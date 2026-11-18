// `@uncheckedVariance` exempts an occurrence from a nested class's constructor's variances however the parameter's
// type reaches it: through an alias selected, inherited, imported, shadowed, in a lambda, in a secondary constructor
// or a bound, inside a context function, a refinement or a wildcard's bound, and on a whole type naming the class's
// own parameters. A written type the resolver does not prove to name classes and type parameters alone is not held.
import scala.annotation.unchecked.uncheckedVariance

// alias_selected
class Outer1[+A] { type U = A @uncheckedVariance; class Inner(x: Outer1.this.U) }
// alias_inherited
trait Base2[+A] { type U = A @uncheckedVariance }
class Outer2[+A] extends Base2[A] { class Inner(x: U) }
// alias_lambda
class Outer3[+A] { type U = A @uncheckedVariance; class Inner(x: ([X] =>> U)[Int]) }
// alias_shadow
class Outer4[+A] { type U = A @uncheckedVariance; class Mid { type U = Int; class Inner(x: Outer4.this.U) } }
// alias_external
class Outer5[+A] { object Types { type U = A @uncheckedVariance }; class Inner(x: Types.U) }
// alias_import
class Outer6[+A] { object Types { type U = A @uncheckedVariance }; import Types.U; class Inner(x: U) }
// alias_secondary
class Outer7[+A] { type U = A @uncheckedVariance; class Inner { def this(x: Outer7.this.U) = this() } }
// alias_bound
class Outer8[+A] { type U = A @uncheckedVariance; class Inner[B <: Outer8.this.U] }
// ctx_annot
class Outer9[+A] { class Inner(x: Int ?=> (A @uncheckedVariance)) }
// refine_annot
class Outer10[+A] { class Inner(x: AnyRef { type T = A @uncheckedVariance }) }
// wild_annot
class Outer11[+A] { class Inner(x: List[? <: (A @uncheckedVariance)]) }
// own_annot
class Outer12[+A] { class Inner[B](x: (A, B) @uncheckedVariance) }
// own_annot_bound
class Outer13[+A] { class Inner[B, C <: ((A, B) @uncheckedVariance)] }
// own_annot_fun
class Outer14[+A] { class Inner[B](x: (B => A) @uncheckedVariance) }

object Main {
  def main(args: Array[String]): Unit = println("built")
}
