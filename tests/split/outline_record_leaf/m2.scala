package m2
import a.*
def other2(): Int = { println("other2"); 2 }
def use2(): Int = g() + f(other2())
