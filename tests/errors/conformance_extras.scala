// After Scala 3's tests/neg/i5094.scala (a final member overridden through the linearisation),
// neg/i10666.scala (bounds of an overriding type parameter), neg/i14187.scala (a protected
// constructor), neg/override-erasure-clash.scala, neg/targetName-override.scala and
// neg/i12682.scala (an inherited member next to an enclosing definition).
// expect: error overriding method c in trait SO of type (x: Int): Int;
// expect: method c in trait SOIO of type (x: Int): Int cannot override final member method c in trait SO
// expect: class Bar needs to be abstract, since def foo[T <: B2](tx: T): Unit in trait Foo is not defined
// expect: constructor Prot cannot be accessed as a member of Prot from class Other
// expect: Name clash between defined and inherited member:
// expect: def f(): Int in class A at line 38 and
// expect: def g(): Int in class B at line 40
// expect: have the same name and type (): Int after erasure.
// expect: should not have a @targetName annotation since the overridden member hasn't one either
// expect: Stable identifier required, but `v` found
// expect: Reference to m is ambiguous.
// expect: It is both defined in object C
// expect: and inherited subsequently in object T
import scala.annotation.targetName
trait IO:
  def c(x: Int): Int = 1
trait SO extends IO:
  override final def c(x: Int): Int = 2
trait SOIO extends IO:
  override def c(x: Int): Int = 3
trait SOSO extends SOIO with SO
abstract class AS extends SO
class L extends AS with SOSO
class A2
class B2 extends A2
trait Foo:
  def foo[T <: B2](tx: T): Unit
class Bar extends Foo:
  def foo[T <: A2](tx: T): Unit = {}
class Prot protected (i: Int)
class Other:
  def make = new Prot(1)
class A:
  def f(): Int = 1
class B extends A:
  @targetName("f") def g(): Int = 2
class Alpha:
  def foo(): Int = 1
class Beta extends Alpha:
  @targetName("foo1") override def foo(): Int = 2
object Pat:
  var v = 1
  def test(x: Int) = x match
    case `v` => 1
    case _ => 2
object C:
  def m(x: Int) = 1
  object T extends K:
    val x = m(1)
class K:
  def m(i: Int) = 2
