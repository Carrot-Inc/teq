// expect: error overriding value p in trait U of type Tiark1.B;
// expect: error overriding value p in trait U of type V.this.X;
// scalac 3.8.4 (tests/neg/i1050a.scala): -- [E164] Declaration Error ...:10:15 error overriding value p in trait U of
// type Tiark1.B; lazy value p of type Tiark1.A & Tiark1.B may not override a non-lazy value; and ...:25:15 the same
// for `lazy val p: X` (RefChecks.checkOverride's lazy-over-strict rule). The gate of 5010f41f8's typing harness:
// different -> accepts. master rejected it by a type mismatch (found Tiark2.v.p.L, required Nothing), its member
// bounds of `p: A & B` taking one side's; the head merges them as dotty's `TypeBounds.&` does, so the bodies type
// and dotty's rejection is RefChecks' (the override checks' part). Pinned for the queued part inlining-lila-folds.
object Tiark1 {
    trait A { type L <: Nothing }
    trait B { type L >: Any}
    trait U {
      val p: B
      def brand(x: Any): p.L = x
    }
    trait V extends U {
      lazy val p: A & B = ???
    }
    val v = new V {}
    v.brand("boom!")
}
object Tiark2 {
    trait A { type L <: Nothing }
    trait B { type L >: Any}
    trait U {
      type X <: B
      val p: X
      def brand(x: Any): p.L = x
    }
    trait V extends U {
      type X = B & A
      lazy val p: X = ???
    }
    val v = new V {}
    v.brand("boom!"): Nothing
}
