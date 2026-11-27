// A local trait's override default read by eight files of the downstream at once: scalac prints 96. The readers'
// contract: tests/modules/efpure/a/A.scala.
@main def run(): Unit = println(Use0.value() + Use1.value() + Use2.value() + Use3.value() + Use4.value() + Use5.value() + Use6.value() + Use7.value())
