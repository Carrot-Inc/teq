// An infix type operator ending its line continues on the next (dotc's newLineOptWhenFollowing over
// canStartInfixTypeTokens), as lila's `type ValidHour = 0 | 1 | ... | 17 |` then `18 | ... | 23`.
object Schedule:
  private type ValidHour = 0 | 1 | 2 | 3 |
    18 | 19 | 23
  type Both = Int &
    Matchable
  def at(hour: ValidHour): Int = hour
  def both(b: Both): Int = b + 1

@main def run(): Unit =
  println(Schedule.at(19))
  println(Schedule.both(2))
