package m1
import a.*
def other1(): Int = { println("other1"); 1 }
def use1(): Int = g() + f(other1())
