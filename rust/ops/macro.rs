use crate::core::VM;

pub struct MacroOpExecutor;

impl MacroOpExecutor {
    pub fn exec(_vm: &mut VM, ko: usize) -> Result<(), ()> {
        let name_macro = [
            "MACRO 1","MACRO 2","MACRO 3","MACRO 4","MACRO 5","MACRO 6","MACRO 7","MACRO 8","MACRO 9",
            "MACRO 10","MACRO 11","MACRO 12","MACRO 13","MACRO 14","MACRO 15","MACRO 16","MACRO 17","MACRO 18",
            "MACRO 19","MACRO 20","MACRO 21","MACRO 22","MACRO 23","MACRO 24","MACRO 25","MACRO 26","MACRO 27"
        ];
        println!(" - [MACRO] : {}", name_macro.get(ko).unwrap_or(&"UNKNOWN"));
        Ok(())
    }
}
