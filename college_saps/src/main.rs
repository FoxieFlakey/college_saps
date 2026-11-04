use libcollege_saps::sap1::SAP1;

fn main() {
    println!("SAP1");
    let mut machine = SAP1::new();
    machine.memory[0x00] = 0b0000_1100;
    machine.memory[0x01] = 0b0010_1011;
    machine.memory[0x02] = 0b0001_1101;
    machine.memory[0x03] = 0b0001_1010;
    machine.memory[0x04] = 0b0010_1110;
    machine.memory[0x05] = 0b0010_1001;
    machine.memory[0x06] = 0b1110_0000;
    machine.memory[0x07] = 0b1111_0000;
    machine.memory[0x08] = 0b0000_0000;
    machine.memory[0x09] = 0b0000_0001;
    machine.memory[0x0A] = 0b0000_0011;
    machine.memory[0x0B] = 0b0000_0010;
    machine.memory[0x0C] = 0b0000_0100;
    machine.memory[0x0D] = 0b0000_0101;
    machine.memory[0x0E] = 0b0000_0110;
    machine.memory[0x0F] = 0b0000_0000;

    while !machine.is_halted {
        machine.advance().expect("Cannot execute the machine");
    }

    println!(
        "Output: Unsigned {0:3}, Signed {1:4} (Hex: 0x{0:02x})",
        machine.out_register,
        i8::from_ne_bytes([machine.out_register])
    );
}
