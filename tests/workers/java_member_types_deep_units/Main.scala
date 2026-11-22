// Eight units, each reading one member type of a chain of sixteen at its own depth (nine to sixteen), from a source:
// every worker count types them as one worker does.
object Main {
  def main(args: Array[String]): Unit = println(U9.v + U10.v + U11.v + U12.v + U13.v + U14.v + U15.v + U16.v - 58)
}
