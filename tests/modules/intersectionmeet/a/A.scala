package ima

trait P:
  def p: Int
trait Q:
  def q: Int
trait A:
  def value(x: Int): P
  def poly[T](t: T): P
trait B:
  def value(x: Int): Q
  def poly[T](t: T): Q
class C extends A with B with P with Q:
  def p = 1
  def q = 2
  def value(x: Int): P & Q = this
  def poly[T](t: T): P & Q = this
