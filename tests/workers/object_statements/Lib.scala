package wos

class Sec0(x: Int) { def this() = this({ println("  unit 0: an object without initialiser was evaluated"); 1 }) }
object Removed0 extends Sec0()
class Loud0 { println("  unit 0: an object with an inherited body") }
object Kept0 extends Loud0
class Arg0(x: Int)
class SecB0(x: Int) { def this() = this({ println("  unit 0: an ascribed block's result was evaluated"); 1 }) }
object Ascribed0 extends SecB0()

class Sec1(x: Int) { def this() = this({ println("  unit 1: an object without initialiser was evaluated"); 1 }) }
object Removed1 extends Sec1()
class Loud1 { println("  unit 1: an object with an inherited body") }
object Kept1 extends Loud1
class Arg1(x: Int)
class SecB1(x: Int) { def this() = this({ println("  unit 1: an ascribed block's result was evaluated"); 1 }) }
object Ascribed1 extends SecB1()

class Sec2(x: Int) { def this() = this({ println("  unit 2: an object without initialiser was evaluated"); 1 }) }
object Removed2 extends Sec2()
class Loud2 { println("  unit 2: an object with an inherited body") }
object Kept2 extends Loud2
class Arg2(x: Int)
class SecB2(x: Int) { def this() = this({ println("  unit 2: an ascribed block's result was evaluated"); 1 }) }
object Ascribed2 extends SecB2()

class Sec3(x: Int) { def this() = this({ println("  unit 3: an object without initialiser was evaluated"); 1 }) }
object Removed3 extends Sec3()
class Loud3 { println("  unit 3: an object with an inherited body") }
object Kept3 extends Loud3
class Arg3(x: Int)
class SecB3(x: Int) { def this() = this({ println("  unit 3: an ascribed block's result was evaluated"); 1 }) }
object Ascribed3 extends SecB3()
