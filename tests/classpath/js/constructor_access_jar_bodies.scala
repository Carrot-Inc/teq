// jars: ctorbodies-lib
// targets: js interp jvm
// teq: --threads 1
// The bodies of a library compiled by scalac create classes with hidden constructors from inside their boundary
// (tests/support/ctor_bodies: a `private[paper]` primary, a private secondary from the companion, protected ones
// through anonymous subclasses, a `@publicInBinary` one in an inline method expanded here, one in a lambda and in
// an inner class): a body read from a jar is asked nothing of them, by the interpreter over the jar, by a build
// that reads the bodies and on the JVM, from an object of this program that extends the library's class.
object Desk:
  private object mill extends paper.Mill(1):
    override def toString = "desk mill"
  def go(): Unit =
    println(mill.stamp(2))
    println(mill.stampOf("abc"))
    println(mill.kept())
    println(mill.keptDefault())
    println(mill.inlineWax(4))
    println(mill.lambda())
    println(new mill.Inner().stamp(5))
    println(paper.Mill.default.stamp(1))
@main def run(): Unit = Desk.go()
