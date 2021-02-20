#include <iostream>
using namespace std;

/* ternary array f_storage[-1:1, -3280:3280, -40:40, 1:6],
                    -- память второго уровня.
                    m_memory[-13:13, -40:40, 1:6],
                    -- память первого уровня.
                    q_swap_storage[-1:1, 1:8],
                    -- указатель (регистр) страницы памяти второго уровня (одного вида), участвующей в обмене.
                    g_io_buffer_reg[-1:1, 1:7],
                    -- буферный регистр для ввода и вывода информации для i-й группы устройст ввода-вывода.
                    u_io_mode[-1:1, 1:4],
                    -- указатель режима работы устройств ввода-вывода i-й группы.
                    h_operands_open_pages[-1:1, 1:3],
                    -- указатель открытых страниц операндов программы.
                    c_mode_and_current_syllable[1:32],
                    -- указатель режимов и выполняемого слога программы.
                    R, Y [1:18],
                    -- R - регистр множителя, Y - регистр младших разрядов.
                    p_stack_pointer[1:10],
                    -- указатель магазина.
                    v_intreq_reg[1:9],
                    -- регистр заявок на прерывание.
                    е_small_adder, k_syllable_reg[1:6],
                    -- e - малый сумматор, k - регистр выполняемого слога.
                    w_interrupt_code[1:4],
                    -- указатель причины прерывания.
                    hf_swap_page[1:3],
                    -- указатель обменной страницы памяти первого уровня.
                    a_io_sync_trigger[-1:1],
                    -- триггер, синхронизирующий работу устройста ввода-вывода i-й грууппы.
                    machine_state [1:1];
                    -- триггер состояния машины.*/

static bool m_state_running = false;

static struct {
	bool m_exception_rised = false;

	unsigned int programmPos = 0;
	unsigned int programmPage = 0;

	unsigned int stackPage = 0;
	unsigned int stackPos = 0;

	unsigned int opcode : 2;
} m_context;

void op_basic(int ko) {
	
	/*
	B1 -- LST
 	B2 -- COT
	B3 -- XNN

	В4 -- Е-1
	B5 -- E=0
	B6 -- Е+1

 	B7 -- T-E
 	B8 -- E=T
 	В9 -- Т+E

	B10 -- CLT
	B11 -- СЕТ
	B12 -- CGT

	B13 -- T=C
	B14 -- R=T
	B15 -- C=T
	B16 -- T=W
	B17 -- YFT

	B18 -- W=S
	В19 -- SMT
	B20 -- Y=T
	B21 -- SAT
	B22 -- S-T
	B23 -- TDN
	В24 -- S+T
	B25 -- LBT
	B26 -- L*T
	B27 -- LHT
	*/
	
	cout << " - [BASIC] " << endl;
}

void op_macro(int ko) {
	
	/**/
	
	cout << " - [MACRO] " << endl;
}

void op_system(int ko) {

	/*
	 S1 -- COPYGl
	 S2 -- COPYG2
	 S3 -- COPYG3

	 S4 -- COPYF1
	 S5 -- COPYF2
	 S6 -- COPYF3

	 S7 -- LOADQ1
	 S8 -- LOADQ2
	 S9 -- LOADQ3

	S10 -- COPYP
	S11 -- EXCHP
	S12 -- LOADP

	S13 -- COPYMC
	S14 -- RETNMC
	S15 -- LOADMC

	S16 -- LOADH1
	S17 -- LOADH2
	S18 -- LOADH3

	S19 -- LOADU1
	S20 -- LOADU2
	S21 -- LOADU3

	S22 -- LOADF1
	S23 -- LOADF2
	S24 -- LOADF3

	S25 -- LOADG1
	S26 -- LOADG2
	S27 -- LOADG3
	*/
	
	cout << " - [SYSTEM] " << endl;
}

void process(int ko) {
	if (m_context.m_exception_rised) {
		m_context.m_exception_rised = false;
	}
}

int execute(int ko) {
	switch (m_context.opcode) {
	case 0:
		op_basic(ko);
		break;

	case 1:
		op_basic(ko);
		break;

	case 2:
		op_basic(ko);
		break;

	default:
		m_context.m_exception_rised = true;
		cout << " - [UNDEFINED] " << endl;
		break;
	}
	return 0;
}

int fetch(void) {
	return 0;
}

int main(int argc, char* argv[]) {

	if (argc > 1) {
		cout << "argc = " << argc << endl;
		
		for (int i = 0; i < argc; ++i) {
			cout << "argv[ " << i << " ] = " << argv[i] << endl << endl;
		}
	}
	else {
		cout << " - No arguments provided" << endl;
		cout << " - Filename: " << argv[0] << endl << endl;
	}

	m_state_running = true;
	while(m_state_running) {
		process(execute(fetch()));
	}

	return 0;
}
