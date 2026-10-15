package siguse
object Use:
  def f = sigprobe.Lib.mix(List(1), List(2), 0)
  def g = sigprobe.Lib.choose(List(1), "x")
  def h = sigprobe.Lib.pick(Set(1), "x")
@main def run(): Unit = println(s"${Use.f} ${Use.g} ${Use.h}")
