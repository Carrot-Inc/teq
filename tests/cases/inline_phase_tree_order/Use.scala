class C:
  def a: Int = M.next
  println(M.next)
@main def run(): Unit = println(new C().a)
