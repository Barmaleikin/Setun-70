#include <iostream>
using namespace std;

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
	cout << " - [BASIC] " << endl;
}

void op_macro(int ko) {
	cout << " - [MACRO] " << endl;
}

void op_system(int ko) {
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
