class OpaqueBox:
  opaque type Value = Int
  opaque type Batch[A] = List[A]
  def make(n: Int): Value = n
  def read(value: Value): Int = value
  def pack[A](values: List[A]): Batch[A] = values
  def first[A](values: Batch[A]): A = values.head

object OpaqueCalls:
  val box = new OpaqueBox
  def direct(value: box.Value): Int = box.read(value)
  def imported(value: box.Value): Int =
    import box.*
    val local: Value = value
    read(local)
  def keep[F[_]](value: F[Int]): F[Int] = value
  def batch(): Int = box.first(keep[box.Batch](box.pack(List(9))))

object OpaqueDerived extends OpaqueBox

@main def run(): Unit =
  println(OpaqueCalls.direct(OpaqueCalls.box.make(3)))
  println(OpaqueCalls.imported(OpaqueCalls.box.make(4)))
  println(OpaqueCalls.batch())
  println(OpaqueDerived.read(OpaqueDerived.make(5)))
