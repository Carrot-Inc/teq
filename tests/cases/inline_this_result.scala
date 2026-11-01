// An inline method's `this` result as the prefix of an inline call in the class's constructor. A plain method's body is
// ascribed its result type (`PrepareInlineable.wrapRHS`), so the prefix has that type: the class's type, an alias of
// `this.type` and an inferred type (widened) keep the proxy, the constructor reading the accessor, the subclass's
// override (2), and so does a body cast or ascribed to `this.type` inside it; a result type of `this.type`, and a
// transparent method's body, which nothing ascribes, are the ThisType (elided: the constructor reads the parameter, 3).
// A body that is another inline method's `this` result, a block or parentheses is ascribed the same way, its own
// method's result type standing over the inner one's (`viaInlT: A = selfT` 2, `viaInlMe: this.type = selfMe` 3),
// unless the outer method is transparent. teq notes the result type as written for the body's root, a deferred call
// among them (`note_wrapped_result`), which a body's copies, a deferred call's node and the retype path keep, the
// outermost note standing; a subclass's `this` keeps the proxy whatever the result type (2).
class A(val s: Int):
  type Me = this.type
  inline def im: Int = s
  inline def self: A = this
  inline def selfT: this.type = this
  inline def selfI = this
  transparent inline def selfTr: A = this
  inline def selfMe: Me = this
  inline def selfAsc: A = (this: this.type)
  inline def selfCastT: A = this.asInstanceOf[this.type]
  inline def bodySelf: Int = self.im
  inline def bodySelfT: Int = selfT.im
  inline def bodySelfTr: Int = selfTr.im
  val same = (this.self eq this)
  val r1 = this.self.im
  val r2 = this.selfT.im
  val r3 = this.selfI.im
  val r4 = this.selfTr.im
  val r5 = this.selfMe.im
  val r6 = this.selfAsc.im
  val r7 = this.selfCastT.im
  val r8 = bodySelf
  val r9 = bodySelfT
  val r10 = bodySelfTr
  inline def selfB: A = { this }
  inline def selfBT: this.type = { this }
  inline def selfP: A = (this)
  inline def viaInlT: A = selfT
  inline def viaInlMe: this.type = selfMe
  inline def viaInlTr: A = selfTr
  transparent inline def trInlT: A = selfT
  transparent inline def trInlSelf: A = self
  inline def viaInlInfer = selfT
  val n1 = selfB.im
  val n2 = selfBT.im
  val n3 = selfP.im
  val n4 = viaInlT.im
  val n5 = viaInlMe.im
  val n6 = viaInlTr.im
  val n7 = trInlT.im
  val n8 = trInlSelf.im
  val n9 = viaInlInfer.im
  inline def body4: Int = viaInlT.im
  inline def body5: Int = viaInlMe.im
  inline def body7: Int = trInlT.im
  val m4 = body4
  val m5 = body5
  val m7 = body7
class B(override val s: Int) extends A(s + 1):
  val q1 = this.self.im
  val q2 = this.selfT.im
  val q3 = this.selfTr.im
  val q4 = bodySelfT
  val q5 = viaInlMe.im
  val q6 = trInlT.im

@main def run(): Unit =
  val b = B(2)
  println(s"result ${b.same} ${b.r1} ${b.r2} ${b.r3} ${b.r4} ${b.r5} ${b.r6} ${b.r7}")
  println(s"body ${b.r8} ${b.r9} ${b.r10}")
  println(s"sub ${b.q1} ${b.q2} ${b.q3} ${b.q4} ${b.q5} ${b.q6}")
  println(s"nested ${b.n1} ${b.n2} ${b.n3} ${b.n4} ${b.n5} ${b.n6} ${b.n7} ${b.n8} ${b.n9}")
  println(s"nested bodies ${b.m4} ${b.m5} ${b.m7}")
