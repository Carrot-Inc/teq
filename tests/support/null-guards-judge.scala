// The judge of tests/support/null_guards.py's verdicts on the listings and modules teq's lowering writes by its own
// null-check rule against those it writes by Scala Native's (the reference): read by tests/native-link.scala and
// tests/native-build.scala, held to planted verdicts by `guardsSelfTest`.

/** The option that lowers by Scala Native's own rule of where a null check is needed: the reference (tests/native-link
 *  holds its output to Scala Native's own bytes). */
val ScalaNativeGuards = "--null-guards=scala-native"

/** What tests/support/null_guards.py found of the listings of teq's own null-check rule against the reference's. */
final case class Guards(equal: Int, added: Int, checks: Int)

val GuardsSummary = "null_guards: ([0-9]+) listings: ([0-9]+) equal, ([0-9]+) by added null checks \\(([0-9]+) checks\\), ([0-9]+) otherwise".r

/** The classifier's verdict on `listings` listings of the default rule's run, or why it does not pass: that run or the
 *  classifier's not ended well (an exit code other than 0, its bound reached), the classifier's self-test line or its
 *  summary missing or twice, a summary of another number of listings or of one otherwise, or whose counts do not add
 *  up to its listings. */
def guardsOf(runStatus: Int, runTimedOut: Boolean, judgeStatus: Int, judgeTimedOut: Boolean, judged: String, listings: Int): Either[List[String], Guards] =
  def ended(what: String, status: Int, timedOut: Boolean) =
    Option.when(status != 0 || timedOut)(s"$what: exit code $status${if timedOut then ", timed out" else ""}")
  val lines = judged.linesIterator.toList
  val summaries = lines.collect { case GuardsSummary(n, e, a, k, o) => (n.toInt, e.toInt, a.toInt, k.toInt, o.toInt) }
  val selfTests = lines.count(_.matches("null_guards self-test: [0-9]+ planted records judged as planted"))
  val read = summaries match
    case List((n, e, a, _, o)) =>
      Option.when(n != listings)(s"the classifier read $n listings of $listings").toList ++
        Option.when(o != 0)(s"$o listings neither equal nor by added null checks").toList ++
        Option.when(e + a + o != n)(s"$e equal, $a by added null checks and $o otherwise of $n listings").toList
    case Nil => List("the classifier's summary missing")
    case more => List(s"the classifier's summary ${more.length} times")
  val problems = ended("the default rule's run", runStatus, runTimedOut).toList ++ ended("the classifier", judgeStatus, judgeTimedOut).toList ++
    Option.when(selfTests != 1)(s"the classifier's self-test line $selfTests times").toList ++ read
  summaries match
    case List((_, e, a, k, _)) if problems.isEmpty => Right(Guards(e, a, k))
    case _ => Left(problems)

/** `guardsOf` held to planted verdicts, each of which has to fail, and sound ones, which have to pass with their
 *  counts: how many of each, and what it misjudges. */
def guardsSelfTest(): (Int, Int, List[String]) =
  val self = "null_guards self-test: 57 planted records judged as planted"
  def said(n: Int, e: Int, a: Int, k: Int, o: Int) = s"null_guards: $n listings: $e equal, $a by added null checks ($k checks), $o otherwise"
  val good = List(self, "added 3 a.ll", said(2, 1, 1, 3, 0)).mkString("\n")
  def judged(text: String, listings: Int = 2, run: Int = 0, runTimedOut: Boolean = false, judge: Int = 0, judgeTimedOut: Boolean = false) =
    guardsOf(run, runTimedOut, judge, judgeTimedOut, text, listings)
  val planted: List[(String, Either[List[String], Guards])] = List(
    "the default rule's run of exit code 1" -> judged(good, run = 1),
    "the default rule's run stopped by its bound" -> judged(good, run = 124, runTimedOut = true),
    "a listing otherwise, the classifier's exit 1" -> judged(List(self, "UNCLASSIFIED a.ll: x", said(2, 1, 0, 0, 1)).mkString("\n"), judge = 1),
    "a listing otherwise, the classifier's exit 0" -> judged(List(self, said(2, 1, 0, 0, 1)).mkString("\n")),
    "the classifier stopped by its bound" -> judged(self, judge = 124, judgeTimedOut = true),
    "the classifier's self-test failed" -> judged("null_guards self-test: FAILED\n  x", judge = 1),
    "the classifier's self-test line missing" -> judged(said(2, 1, 1, 3, 0)),
    "the classifier's summary missing" -> judged(self),
    "the classifier's summary twice" -> judged(List(self, said(2, 1, 1, 3, 0), said(2, 1, 1, 3, 0)).mkString("\n")),
    "a listing missing (the classifier read one fewer)" -> judged(List(self, said(1, 0, 1, 3, 0)).mkString("\n")),
    "a listing more than the records" -> judged(List(self, said(3, 2, 1, 3, 0)).mkString("\n")),
    "counts that do not add up to the listings" -> judged(List(self, said(2, 1, 0, 0, 0)).mkString("\n")),
    "a summary of another form" -> judged(List(self, "null_guards: 2 listings: 1 equal, 1 by added null checks, 0 otherwise").mkString("\n")))
  val soundOnes: List[(String, Either[List[String], Guards], Guards)] = List(
    ("two modules, one by added null checks", judged(good), Guards(1, 1, 3)),
    ("a listing equal", judged(List(self, said(1, 1, 0, 0, 0)).mkString("\n"), listings = 1), Guards(1, 0, 0)))
  (planted.size, soundOnes.size, planted.collect { case (what, Right(g)) => s"the classifier's judge passes $what: $g" } ++
    soundOnes.collect { case (what, got, want) if got != Right(want) => s"the classifier's judge on $what: $got, not $want" })
