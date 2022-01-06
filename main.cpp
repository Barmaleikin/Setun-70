#include <iostream>
#include <iomanip>
#include <string>
#include <array>

using namespace std;

const int valRange = 364;
struct anValue {
    short valBinary = -364;
    short valTrinary = 0;

    std::string txtBinary = "0";
    std::string txtTrinary = "0";

    bool isZero = true;
    bool isPositive = false;
    bool isEven = false;
} m_Val;
anValue _tmp[valRange * 2 + 1];
anValue* mArray = _tmp + valRange;

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

enum mMachineMode {
	mm_BASIC, mm_SYSTEM, mm_USER
};

void op_basic(int ko) {

	enum opBasic { LST, COT, XNN, DOW, BRT, JMP, T_sub_E, E_setto_T, T_add_E,
               CLT, CET, CGT, JSR, R_setto_T, C_setto_T, T_setto_W, YFT, W_setto_S,
               SMT, Y_setto_T, SAT, S_sub_T, TDN, S_add_T, LBT, L_mul_T, LHT };

	const string nameBasic[] = { "LST", "COT", "XNN", "DOW", "BRT", "JMP", "T-E", "E=T", "T+E",
                         "CLT", "CET", "CGT", "JSR", "R=T", "C=T", "T=W", "YFT", "W=S",
                         "SMT", "Y=T", "SAT", "S-T", "TDN", "S+T", "LBT", "L*T", "LHT" };
	
	switch (ko) {

	    case opBasic::LST:
		break;

	    case opBasic::COT:
		break;

	    case opBasic::XNN:
		break;

	    case opBasic::DOW:
		break;

	    case opBasic::BRT:
		break;

	    case opBasic::JMP:
		break;

	    case opBasic::T_sub_E:
		break;

	    case opBasic::E_setto_T:
		break;

	    case opBasic::T_add_E:
		break;

	    case opBasic::CLT:
		break;

	    case opBasic::CET:
		break;

	    case opBasic::CGT:
		break;

	    case opBasic::JSR:
		break;

	    case opBasic::R_setto_T:
		break;

	    case opBasic::C_setto_T:
		break;

	    case opBasic::T_setto_W:
		break;

	    case opBasic::YFT:
		break;

	    case opBasic::W_setto_S:
		break;

	    case opBasic::SMT:
		break;

	    case opBasic::Y_setto_T:
		break;

	    case opBasic::SAT:
		break;

	    case opBasic::S_sub_T:
		break;

	    case opBasic::TDN:
		break;

	    case opBasic::S_add_T:
		break;

	    case opBasic::LBT:
		break;

	    case opBasic::L_mul_T:
		break;

	    case opBasic::LHT:
		break;
	}
	
	cout << " - [BASIC] " << endl;
}

void op_macro(int ko) {
	
	enum opMacro {
	    MACRO1, MACRO2, MACRO3, MACRO4, MACRO5, MACRO6, MACRO7, MACRO8, MACRO9,
	    MACRO10, MACRO11, MACRO12, MACRO13, MACRO14, MACRO15, MACRO16, MACRO17, MACRO18,
	    MACRO19, MACRO20, MACRO21, MACRO22, MACRO23, MACRO24, MACRO25, MACRO26, MACRO27
	};

	const string nameMacro[] = { "MACRO 1", "MACRO 2", "MACRO 3", "MACRO 4", "MACRO 5", "MACRO 6", "MACRO 7", "MACRO 8", "MACRO 9",
				"MACRO 10", "MACRO 11", "MACRO 12", "MACRO 13", "MACRO 14", "MACRO 15", "MACRO 16", "MACRO 17", "MACRO 18",
				"MACRO 19", "MACRO 20", "MACRO 21", "MACRO 22", "MACRO 23", "MACRO 24", "MACRO 25", "MACRO 26", "MACRO 27" };

	    switch (ko) {

	    case opMacro::MACRO1:
		break;

	    case opMacro::MACRO2:
		break;

	    case opMacro::MACRO3:
		break;

	    case opMacro::MACRO4:
		break;

	    case opMacro::MACRO5:
		break;

	    case opMacro::MACRO6:
		break;

	    case opMacro::MACRO7:
		break;

	    case opMacro::MACRO8:
		break;

	    case opMacro::MACRO9:
		break;

	    case opMacro::MACRO10:
		break;

	    case opMacro::MACRO11:
		break;

	    case opMacro::MACRO12:
		break;

	    case opMacro::MACRO13:
		break;

	    case opMacro::MACRO14:
		break;

	    case opMacro::MACRO15:
		break;

	    case opMacro::MACRO16:
		break;

	    case opMacro::MACRO17:
		break;

	    case opMacro::MACRO18:
		break;

	    case opMacro::MACRO19:
		break;

	    case opMacro::MACRO20:
		break;

	    case opMacro::MACRO21:
		break;

	    case opMacro::MACRO22:
		break;

	    case opMacro::MACRO23:
		break;

	    case opMacro::MACRO24:
		break;

	    case opMacro::MACRO25:
		break;

	    case opMacro::MACRO26:
		break;

	    case opMacro::MACRO27:
		break;
	    }
	
	cout << " - [MACRO] " << endl;
}

void op_system(int ko) {

	enum opSupervisor { CG1, CG2, CG3, CF1, CF2, CF3, LQ1, LQ2, LQ3,
                    CP, EXP, LP, CMC, RMC, LMC, LH1, LH2, LH3,
                    LU1, LU2, LU3, LF1, LF2, LF3, LG1, LG2, LG3};

	const string nameSpecial[] = { "COPYG1", "COPYG2", "COPYG3", "COPYF1", "COPYF2", "COPYF3", "LOADQ1", "LOADQ2", "LOADQ3",
                               "COPYP", "EXCHP", "LOADP", "COPYMC", "RETNMC", "LOADMC", "LOADH1", "LOADH2", "LOADH3",
                               "LOADU1", "LOADU2", "LOADU3", "LOADF1", "LOADF2", "LOADF3", "LOADG1", "LOADG2", "LOADG3" };
	
	switch (ko) {

	    case opSupervisor::CG1:
		break;

	    case opSupervisor::CG2:
		break;

	    case opSupervisor::CG3:
		break;

	    case opSupervisor::CF1:
		break;

	    case opSupervisor::CF2:
		break;

	    case opSupervisor::CF3:
		break;

	    case opSupervisor::LQ1:
		break;

	    case opSupervisor::LQ2:
		break;

	    case opSupervisor::LQ3:
		break;

	    case opSupervisor::CP:
		break;

	    case opSupervisor::EXP:
		break;

	    case opSupervisor::LP:
		break;

	    case opSupervisor::CMC:
		break;

	    case opSupervisor::RMC:
		break;

	    case opSupervisor::LMC:
		break;

	    case opSupervisor::LH1:
		break;

	    case opSupervisor::LH2:
		break;

	    case opSupervisor::LH3:
		break;

	    case opSupervisor::LU1:
		break;

	    case opSupervisor::LU2:
		break;

	    case opSupervisor::LU3:
		break;

	    case opSupervisor::LF1:
		break;

	    case opSupervisor::LF2:
		break;

	    case opSupervisor::LF3:
		break;

	    case opSupervisor::LG1:
		break;

	    case opSupervisor::LG2:
		break;

	    case opSupervisor::LG3:
		break;
	}
	
	cout << " - [SYSTEM] " << endl;
}

void process(int ko) {
	if (m_context.m_exception_rised) {
		m_context.m_exception_rised = false;
	}
}

int execute(int ko) {

	enum opcodes {
		mCodeBasic, mCodeSystem, mCodeMacro
	};

	switch (m_context.opcode) {
	case opcodes::mCodeBasic:
		op_basic(ko);
		break;

	case opcodes::mCodeSystem:
		op_system(ko);
		break;

	case opcodes::mCodeMacro:
		op_macro(ko);
		break;

	default:
		m_context.m_exception_rised = true;
		cout << " - [UNDEFINED] " << endl;
		break;
	}
	return 0;
}

int fetch(void) {
	m_context.opcode = 0;
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
