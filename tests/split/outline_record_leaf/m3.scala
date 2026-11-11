package m3
import a.*
def other3(): Int = { println("other3"); 3 }
def use3(): Int = g() + f(other3())
