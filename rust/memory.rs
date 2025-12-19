use std::fmt;

#[derive(Debug)]
pub enum MemoryError {
    PageOutOfRange(i8),
    OffsetOutOfRange(usize),
    CountOutOfRange(usize),
    StackOverflow,
    StackUnderflow,
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryError::PageOutOfRange(p) => write!(f, "Page out of range: {}", p),
            MemoryError::OffsetOutOfRange(o) => write!(f, "Offset out of range: {}", o),
            MemoryError::CountOutOfRange(c) => write!(f, "Count out of range: {}", c),
            MemoryError::StackOverflow => write!(f, "Stack overflow"),
            MemoryError::StackUnderflow => write!(f, "Stack underflow"),
        }
    }
}

pub struct Memory {
    pages: Vec<[i32; Self::PAGE_SIZE]>, // 27 страниц по 81 числу
    pub current_page: i8,               // регистр текущей страницы (-13..+13)
    pub current_triplet_index: usize,   // индекс тройки внутри страницы (0..26)
    pub current_within_triplet: usize,  // позиция внутри тройки (0..2) для поэлементного чтения
}

impl Memory {
    pub const PAGE_COUNT: usize = 27;
    pub const PAGE_SIZE: usize = 81;
    pub const TRIPLET_SIZE: usize = 3;
    pub const TRIPLETS_PER_PAGE: usize = Self::PAGE_SIZE / Self::TRIPLET_SIZE; // 27
    pub const FIRST_PAGE_NUM: i8 = -13;
    pub const LAST_PAGE_NUM: i8 = 13;

    pub fn new() -> Self {
        Self {
            pages: vec![[0i32; Self::PAGE_SIZE]; Self::PAGE_COUNT],
            current_page: 0,
            current_triplet_index: 0,
            current_within_triplet: 0,
        }
    }

    fn page_index(page: i8) -> Result<usize, MemoryError> {
        if page < Self::FIRST_PAGE_NUM || page > Self::LAST_PAGE_NUM {
            return Err(MemoryError::PageOutOfRange(page));
        }
        Ok((page - Self::FIRST_PAGE_NUM) as usize)
    }

    fn can_write(page: i8) -> bool {
        (-4..=4).contains(&page)
    }

    // Прямое чтение count чисел с выравниванием по троикам (сохранил для совместимости)
    pub fn read_page(&self, page: i8, offset: usize, count: usize) -> Result<Vec<i32>, MemoryError> {
        let idx = Self::page_index(page)?;
        if offset >= Self::PAGE_SIZE {
            return Err(MemoryError::OffsetOutOfRange(offset));
        }
        if count == 0 {
            return Ok(vec![]);
        }
        if offset + count > Self::PAGE_SIZE {
            return Err(MemoryError::CountOutOfRange(count));
        }

        let mut result: Vec<i32> = Vec::with_capacity(((count + 2) / 3) * 3);
        let page_ref = &self.pages[idx];
        for i in 0..count {
            result.push(page_ref[offset + i]);
        }
        while result.len() % 3 != 0 {
            result.push(0);
        }
        Ok(result)
    }

    pub fn write_page(&mut self, page: i8, offset: usize, values: &[i32]) -> Result<(), MemoryError> {
        let idx = Self::page_index(page)?;
        if offset >= Self::PAGE_SIZE {
            return Err(MemoryError::OffsetOutOfRange(offset));
        }

        let mut to_write: Vec<i32> = values.to_vec();
        while to_write.len() % 3 != 0 {
            to_write.push(0);
        }
        if offset + to_write.len() > Self::PAGE_SIZE {
            return Err(MemoryError::CountOutOfRange(to_write.len()));
        }

        // Если запись запрещена — не сохраняем, но считаем операцию успешной
        if !Self::can_write(page) {
            return Ok(());
        }

        let page_ref = &mut self.pages[idx];
        for (i, v) in to_write.iter().enumerate() {
            page_ref[offset + i] = *v;
        }
        Ok(())
    }

    pub fn read_triplet_at(&self, page: i8, triplet_index: usize) -> Result<[i32; 3], MemoryError> {
        if triplet_index >= Self::TRIPLETS_PER_PAGE {
            return Err(MemoryError::OffsetOutOfRange(triplet_index * Self::TRIPLET_SIZE));
        }
        let start = triplet_index * Self::TRIPLET_SIZE;
        let idx = Self::page_index(page)?;
        let page_ref = &self.pages[idx];
        Ok([
            page_ref[start],
            page_ref[start + 1],
            page_ref[start + 2],
        ])
    }

    pub fn write_triplet_at(&mut self, page: i8, triplet_index: usize, triple: [i32; 3]) -> Result<(), MemoryError> {
        if triplet_index >= Self::TRIPLETS_PER_PAGE {
            return Err(MemoryError::OffsetOutOfRange(triplet_index * Self::TRIPLET_SIZE));
        }
        let start = triplet_index * Self::TRIPLET_SIZE;
        self.write_page(page, start, &triple)
    }

    // --- Стековые операции (push сохраняет тройку или пропускает запись по запрету, но всегда двигает указатель) ---
    pub fn push_triplet(&mut self, triple: [i32; 3]) -> Result<(), MemoryError> {
        if self.current_triplet_index >= Self::TRIPLETS_PER_PAGE {
            return Err(MemoryError::StackOverflow);
        }

        // Пытаемся записать (write_page вернёт Ok и не сохранит, если страница запрещена)
        let _ = self.write_triplet_at(self.current_page, self.current_triplet_index, triple);

        // Смещаем указатель на следующую тройку
        self.current_triplet_index += 1;
        self.current_within_triplet = 0;

        if self.current_triplet_index >= Self::TRIPLETS_PER_PAGE {
            let next_page = self.current_page.checked_add(1).ok_or(MemoryError::StackOverflow)?;
            if next_page > Self::LAST_PAGE_NUM {
                return Err(MemoryError::StackOverflow);
            }
            self.current_page = next_page;
            self.current_triplet_index = 0;
        }
        Ok(())
    }

    // Поп: откатываем указатель и читаем тройку целиком
    pub fn pop_triplet(&mut self) -> Result<[i32; 3], MemoryError> {
        if self.current_within_triplet != 0 {
            // если были частичные чтения внутри тройки — приводим к началу следующей тройки для корректного pop
            self.current_within_triplet = 0;
        }

        if self.current_triplet_index == 0 {
            if self.current_page == Self::FIRST_PAGE_NUM {
                return Err(MemoryError::StackUnderflow);
            }
            self.current_page = self.current_page.checked_sub(1).ok_or(MemoryError::StackUnderflow)?;
            self.current_triplet_index = Self::TRIPLETS_PER_PAGE;
        }
        if self.current_triplet_index == 0 {
            return Err(MemoryError::StackUnderflow);
        }
        self.current_triplet_index -= 1;
        let triple = self.read_triplet_at(self.current_page, self.current_triplet_index)?;
        self.current_within_triplet = 0;
        Ok(triple)
    }

    // --- Новое: поэлементное чтение "read_one" — возвращает одно число из текущей тройки и продвигает указатель внутри тройки/между тройками ---
    pub fn read_one(&mut self) -> Result<i32, MemoryError> {
        // Проверки границ
        if self.current_triplet_index >= Self::TRIPLETS_PER_PAGE {
            return Err(MemoryError::OffsetOutOfRange(self.current_triplet_index * Self::TRIPLET_SIZE));
        }
        // Получаем индекс страницы в векторе
        let pidx = Self::page_index(self.current_page)?;
        let start = self.current_triplet_index * Self::TRIPLET_SIZE;
        let page_ref = &self.pages[pidx];

        let value = page_ref[start + self.current_within_triplet];

        // Продвинуть позицию внутри тройки
        self.current_within_triplet += 1;
        if self.current_within_triplet >= Self::TRIPLET_SIZE {
            // перейти на следующую тройку
            self.current_within_triplet = 0;
            self.current_triplet_index += 1;
            if self.current_triplet_index >= Self::TRIPLETS_PER_PAGE {
                // перейти на следующую страницу
                let next_page = self.current_page.checked_add(1).ok_or(MemoryError::StackOverflow)?;
                if next_page > Self::LAST_PAGE_NUM {
                    return Err(MemoryError::StackOverflow);
                }
                self.current_page = next_page;
                self.current_triplet_index = 0;
            }
        }
        Ok(value)
    }

    // Установить указатель вручную
    pub fn set_pointer(&mut self, page: i8, triplet_index: usize, within_triplet: usize) -> Result<(), MemoryError> {
        Self::page_index(page)?;
        if triplet_index >= Self::TRIPLETS_PER_PAGE {
            return Err(MemoryError::OffsetOutOfRange(triplet_index * Self::TRIPLET_SIZE));
        }
        if within_triplet >= Self::TRIPLET_SIZE {
            return Err(MemoryError::OffsetOutOfRange(within_triplet));
        }
        self.current_page = page;
        self.current_triplet_index = triplet_index;
        self.current_within_triplet = within_triplet;
        Ok(())
    }

    pub fn get_pointer(&self) -> (i8, usize, usize) {
        (self.current_page, self.current_triplet_index, self.current_within_triplet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_one_basic() {
        let mut m = Memory::new();
        // запишем одну тройку на страницу 0, тройка 0
        m.push_triplet([11,22,33]).unwrap();
        // сдвинем указатель назад чтобы читать с начала
        m.set_pointer(0, 0, 0).unwrap();
        assert_eq!(m.read_one().unwrap(), 11);
        assert_eq!(m.read_one().unwrap(), 22);
        assert_eq!(m.read_one().unwrap(), 33);
        // После чтения указатель должен указывать на следующую тройку (0,1,0)
        assert_eq!(m.get_pointer(), (0,1,0));
    }

    #[test]
    fn skip_write_behavior() {
        let mut m = Memory::new();
        // поставим указатель на страницу 5 (запись запрещена)
        m.set_pointer(5, 0, 0).unwrap();
        m.push_triplet([1,2,3]).unwrap(); // не сохранится, но pointer -> (5,1,0)
        m.set_pointer(5, 0, 0).unwrap();
        // read_one вернёт начальное значение (0), потому что данные не записывались
        assert_eq!(m.read_one().unwrap(), 0);
    }

    #[test]
    fn crossing_pages() {
        let mut m = Memory::new();
        // заполним последнюю тройку страницы 0
        m.set_pointer(0, Memory::TRIPLETS_PER_PAGE - 1, 2).unwrap(); // внутри последней тройки, позиция 2
        // чтение последнего элемента приведёт к переходу на страницу 1, тройка 0, pos 0
        // убеждаемся, что в пределах допустимых страниц
        let _ = m.read_one();
        let (p, t, w) = m.get_pointer();
        assert!(p >= 0 && p <= Memory::LAST_PAGE_NUM);
        assert_eq!(w, 0);
    }
}
