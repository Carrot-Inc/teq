// The jar side of tests/cases/library_val_overridden, compiled by scalac: a class's vals that a
// subclass overrides, scalacheck's `Test.Parameters` and its `cpy`'s anonymous class among them, and a
// val that implements a def (read through an accessor method, izumi's `override val hashCode`).
package lvlib

abstract class Params:
  val minSize: Int
  val legacy: Boolean = true
  val spins: Int = 1
  def describe: String = s"$minSize $legacy $spins"
  def withLegacy(b: Boolean): Params = Params.cpy(minSize, b)

object Params:
  def cpy(size: Int, b: Boolean): Params = new Params:
    val minSize: Int = size
    override val legacy: Boolean = b
  val default: Params = new Params:
    val minSize: Int = 0

trait Labeled:
  def label: String

abstract class Badge extends Labeled:
  val label: String = "badge"
  override val hashCode: Int = 41
  def show: String = s"$label $hashCode"
