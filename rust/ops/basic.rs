use crate::core::VM;

pub struct BasicOpExecutor;

impl BasicOpExecutor {
    pub fn exec(vm: &mut VM, ko: usize) -> Result<(), ()> {
        match ko {
            17 => Self::op_yft(vm),
            19 => Self::op_y_setto_t(vm),
            21 => Self::op_s_sub_t(vm),
            22 => Self::op_tdn(vm),
            23 => Self::op_s_add_t(vm),
            _ => { /* not implemented */ }
        }

        let name_basic = [
            "LST","COT","XNN","DOW","BRT","JMP","T-E","E=T","T+E",
            "CLT","CET","CGT","JSR","R=T","C=T","T=W","YFT","W=S",
            "SMT","Y=T","SAT","S-T","TDN","S+T","LBT","L*T","LHT"
        ];
        let name = name_basic.get(ko).unwrap_or(&"UNKNOWN");
        println!(" - [BASIC] : {}", name);
        Ok(())
    }

    fn op_yft(vm: &mut VM) {
        vm.reg_y = VM::val_index(0);
    }

    fn op_y_setto_t(vm: &mut VM) {
        if let Some(top) = vm.pop() {
            let idx = VM::val_index(top.val_binary);
            vm.reg_y = idx;
        }
    }

    fn op_s_sub_t(vm: &mut VM) {
        let a = vm.pop().map(|v| v.val_binary).unwrap_or(0);
        let b = vm.pop().map(|v| v.val_binary).unwrap_or(0);
        let result = VM::wrap_value(a as i32 - b as i32);
        vm.push(result);
    }

    fn op_tdn(vm: &mut VM) {
        if let Some(top) = vm.pop() {
            let temp = -top.val_binary;
            vm.push(temp);
        }
    }

    fn op_s_add_t(vm: &mut VM) {
        let a = vm.pop().map(|v| v.val_binary).unwrap_or(0);
        let b = vm.pop().map(|v| v.val_binary).unwrap_or(0);
        let result = VM::wrap_value(a as i32 + b as i32);
        vm.push(result);
    }
}
