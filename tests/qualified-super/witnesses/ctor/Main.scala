class A { def m(): String = "A" }
class Parent(val x: String)
class B extends A { class I extends Parent(B.super.m()) }
@main def run(): Unit = { val b = new B; println((new b.I).x) }
