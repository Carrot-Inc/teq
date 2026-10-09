package errs

trait Base:
  def name: String

trait Loud extends Base:
  abstract override def name: String = super.name + "!"

class Impl extends Base:
  val name: String = "x"

/** A super call of a parent that binds to a value: told once per class. */
class K extends Impl with Loud:
  def label: String = "k"
