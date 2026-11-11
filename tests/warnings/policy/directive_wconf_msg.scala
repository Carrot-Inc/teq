//> using options "-Wconf:msg=pure expression:s"
object O {
  @deprecated("old", "1") def old(): Int = 1
  def f(): Unit = { 1; () }
}
@main def run(): Unit = { O.f(); println(O.old()) }
