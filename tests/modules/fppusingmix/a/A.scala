package usingmix
trait B(using val x: Int)(val y: Int = 7)
trait T extends B
given Int = 31
class K extends T
