// A private constructor var assigned through another instance by a compound assignment is variance-checked at an
// invariant position, as a plain assignment through it is.
class C[+T](private var row: List[T]):
  def act(o: C[Any]): Unit = o.row ++= Nil
  def size: Int = row.size
@main def run(): Unit = println(new C[Int](List(1)).size)
// expect: 3:25: error: covariant type T occurs in invariant position in type List[T] of variable row
