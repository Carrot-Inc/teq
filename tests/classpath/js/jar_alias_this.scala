// jars: ctor-lib scala-library
// targets: js interp jvm
// The inline `this` decision on a body loaded from a jar (tests/support/ctor_lib.scala's `AliasThis`): a cast or an
// ascription of `this` to `this.type` is the ThisType (elided: the constructor reads the parameter, 3), one to an alias
// of it (`type Me = this.type`) keeps the proxy (the accessor, the subclass's override, 2), in the jar's constructor
// and in its inline methods' bodies the constructor calls. The pickle's type tree is a ThisType or a `TYPEREF` to the
// alias, which the loader notes before the store expands the alias (`Ast::this_type_trees`). A subclass's `this`
// calling those methods keeps the proxy (2), and its own cast to `this.type` is its ThisType, not `A`'s (2). An inline
// method's `this` result is a prefix of its result type, the pickle's body ascribed to it
// (`PrepareInlineable.wrapRHS`): the class's type keeps the proxy (2), `this.type` and a transparent method's body are
// the ThisType (3), and a body that is another inline method's result takes its own method's result type (`AliasThis`
// over `this.type`: 2; `this.type` over an alias: 3). The jar's macro (`AliasMac.keep`) splices a source class's
// inline call back through its quote, decided as from source.
class AliasSub(override val s: Int) extends ctorlib.AliasThis(s + 1):
  val subMe = viaMe
  val subThis = viaThis
  val subCast = this.asInstanceOf[this.type].im
  val subSelf = this.self.im

class QA(val s: Int):
  type Me = this.type
  inline def im: Int = s
  inline def self: QA = this
  inline def selfT: this.type = this
  val q1 = ctorlib.AliasMac.keep(this.asInstanceOf[this.type].im)
  val q2 = ctorlib.AliasMac.keep(this.asInstanceOf[Me].im)
  val q3 = ctorlib.AliasMac.keep((this: Me).im)
  val q4 = ctorlib.AliasMac.keep((this: this.type).im)
  val q6 = ctorlib.AliasMac.keep(this.self.im)
  val q7 = ctorlib.AliasMac.keep(this.selfT.im)
class QB(override val s: Int) extends QA(s + 1):
  val q5 = ctorlib.AliasMac.keep(this.im)

@main def run(): Unit =
  val b = AliasSub(2)
  println(s"jar ${b.aliased} ${b.direct} ${b.ascAlias} ${b.ascDirect}")
  println(s"jar bodies ${b.nestedMe} ${b.nestedThis} ${b.nestedAscMe} ${b.nestedAscThis}")
  println(s"jar results ${b.resSelf} ${b.resSelfT} ${b.resSelfTr} ${b.resInlT} ${b.resInlMe}")
  println(s"sub ${b.subMe} ${b.subThis} ${b.subCast} ${b.subSelf}")
  val q = QB(2)
  println(s"jar quoted ${q.q1} ${q.q2} ${q.q3} ${q.q4} sub ${q.q5} results ${q.q6} ${q.q7}")
