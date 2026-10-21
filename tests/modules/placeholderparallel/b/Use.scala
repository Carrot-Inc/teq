package pb

object Use:
  def run(): List[Int] = pa.Ops.twice(pa.Ops.inc(List(1, 2))) :+ pa.Ops.sum

@main def main(): Unit = println(Use.run())
