use crate::core::VM;

pub struct SystemOpExecutor;

impl SystemOpExecutor {
    pub fn exec(_vm: &mut VM, ko: usize) -> Result<(), ()> {
        let name_system = [
            "COPYG1","COPYG2","COPYG3","COPYF1","COPYF2","COPYF3","LOADQ1","LOADQ2","LOADQ3",
            "COPYP","EXCHP","LOADP","COPYMC","RETNMC","LOADMC","LOADH1","LOADH2","LOADH3",
            "LOADU1","LOADU2","LOADU3","LOADF1","LOADF2","LOADF3","LOADG1","LOADG2","LOADG3"
        ];
        println!(" - [SYSTEM] : {}", name_system.get(ko).unwrap_or(&"UNKNOWN"));
        Ok(())
    }
}
