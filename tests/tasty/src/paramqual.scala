// Constructor accessors qualified `private[shapes]` and `protected[shapes]`, which TASTy keeps as the accessors'
// `PRIVATEqualified` and `PROTECTEDqualified` (dotty's `privateWithin`): outside the package a subclass's `override val`
// is no inherited accessor of the private one and a selection of it is refused, while patterns, `copy` and a
// subclass's read of the protected one stay (tests/classpath/js/jar_param_qualified.scala,
// tests/classpath/jar_param_qualified_refused.scala); and a private parameter passed on unchanged to its
// superclass's like-named one (`PARAMalias`, tests/classpath/pending/jar_param_alias.scala); a case class's private
// accessor, `PRIVATE` beside `CASEaccessor` (tests/classpath/pending/jar_case_private_accessor.scala).
package fix.shapes

class PqA(private[shapes] val s: Int)
case class PqK(private[shapes] val x: Int, y: Int)
class PqH(protected[shapes] val h: Int)
class PqLA(val s: Int)
class PqLB(s: Int) extends PqLA(s)
case class PqCase(private val s: Int)
