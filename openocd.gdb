target remote :3333
set print asm-demangle on
set print pretty on
monitor arm semihosting enable
load
break DefaultHandler
break HardFault
continue
