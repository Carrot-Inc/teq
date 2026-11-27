object Narrow {
  def run(): String = {
    class C { def m[A](a: A = "yes"): A = a }
    new C().m[String]()
  }
}
