class A { def m(): String = "A" }
class B extends A { def f = (() => super.m())(); def g = (() => B.super.m())() }
@main def run(): Unit = { val b = new B; println(b.f); println(b.g) }
