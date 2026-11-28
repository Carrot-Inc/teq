abstract class A { def m(): String }
class B extends A { override def m() = "B"; class I { def f = B.super.m() } }
