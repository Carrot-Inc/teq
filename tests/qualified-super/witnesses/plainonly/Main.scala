package sites
class A { def m(): String = "A" }
class Plain extends A {
 def local = { def f = super.m(); f }
 def lambda = (() => super.m())()
 def qualified = (() => Plain.super.m())()
 def byname = { def take(x: => String) = x; take(super.m()) }
 val field = super.m()
 lazy val lazyfield = super.m()
 def default(x: String = super.m()) = x
 def guard = 1 match { case x if super.m() == "A" => "A"; case _ => "bad" }
}

@main def run(): Unit = { val p = new Plain; println(p.local); println(p.lambda); println(p.qualified); println(p.byname); println(p.field); println(p.lazyfield); println(p.default()); println(p.guard) }
