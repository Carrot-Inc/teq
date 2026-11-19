package pk
package inner

object Mine:
  def get: String = s"${summon[P].x}${summon[Q].q}"

@main def run(): Unit =
  println(s"3.6: ${Old.get}; default: ${Mine.get}")
