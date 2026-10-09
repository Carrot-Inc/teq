// teq: --werror
// An irrefutable extractor whose sub-pattern leaves part of its field uncovered does not cover
// the scrutinee.
enum Day:
  case Mon, Tue
def name(p: (Day, Int)): String = p match
  case Day.Mon -> n => s"mon $n"
@main def run(): Unit = println(name(Day.Mon -> 1))
// expect: match may not be exhaustive
