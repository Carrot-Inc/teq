package m4
import a.*
def other4(): Int = { println("other4"); 4 }
def use4(): Int = g() + f(other4())
