// A class parameter written protected[this] is variance-checked as a var, at an invariant position.
class C[+T](protected[this] var row: List[T]):
  def size: Int = row.size
@main def run(): Unit = println(new C[Int](List(1)).size)
// expect: 2:33: error: covariant type T occurs in invariant position in type List[T] of variable row
