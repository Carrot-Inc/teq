package a2
class C { def n: Int = 31 }
trait T extends C
given gp(using k: Int): T with {}
given Int = 3
