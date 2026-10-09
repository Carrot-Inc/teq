import scala.util.{Failure, Success, Try}

def show(label: String, values: Any*): Unit = println(label + ": " + values.mkString(" | "))

def parsePort(s: String): Try[Int] = Try(s.toInt).filter(p => p > 0 && p < 65536)

def describe(t: Try[Int]): String = t match
  case Success(v) => "ok " + v.toString
  case Failure(e) => "failed"

@main def main(): Unit =
  show("toOption", Try("12".toInt).toOption, Try("x".toInt).toOption, Try("1.5".toDouble).toOption, Try("".toDouble).toOption)
  show("state", Try(1).isSuccess, Try(1).isFailure, Try(sys.error("boom")).isFailure, Try(List(1)(5)).isFailure, Try(1 / 0).isFailure)
  show("message", Try(sys.error("boom")).failed.map(_.getMessage), Try[Int](sys.error("boom")).toEither.left.map(_.getMessage))
  show("getOrElse", Try("7".toInt).getOrElse(0), Try("z".toInt).getOrElse(0), Try("z".toInt).orElse(Try(5)), Try(1).orElse(Try(2)))
  show("map", Try(2).map(_ * 3), Try(2).map(x => x / 0).isFailure, Try("q".toInt).map(_ + 1).isFailure)
  show("flatMap", Try(2).flatMap(x => Try(x + 1)), Try(2).flatMap(x => Try("n".toInt)).isFailure, Try(2).flatMap(x => sys.error("in f")).isFailure)
  show("for",
    (for
      a <- Try("4".toInt)
      b <- Try("5".toInt)
    yield a * b),
    (for
      a <- Try("4".toInt)
      b <- Try("five".toInt)
    yield a * b).isFailure)
  show("filter", parsePort("8080"), parsePort("0").isFailure, parsePort("http").isFailure)
  show("fold", Try(3).fold(_ => -1, _ * 2), Try("x".toInt).fold(_ => -1, _ * 2), Try(3).fold(_ => -1, x => x / 0))
  show("match", describe(Try(3)), describe(Try("x".toInt)), describe(Success(9)))
  show("toEither", Try(3).toEither, Try("x".toInt).toEither.isLeft, Try(3).toEither.toOption)
  show("flatten", Try(Try(1)).flatten, Try(Try("x".toInt)).flatten.isFailure)
  show("transform", Try(1).transform(v => Success(v + 1), _ => Success(0)), Try("x".toInt).transform(v => Success(v + 1), _ => Success(0)))
  show("failed", Try(1).failed.isFailure, Try("x".toInt).failed.isSuccess)
  show("foreach", { var seen = 0; Try(5).foreach(seen += _); Try("x".toInt).foreach(seen += _); seen })
  show("get", Try(10).get, Success("v").value)
  var effects = 0
  val discarded = Try({ effects += 1; "x".toInt })
  show("eager", effects, discarded.isFailure)
  show("either toTry", Right[Throwable, Int](1).toTry, Try("1".toInt).toEither.toTry)
  show("options", List("1", "x", "3").flatMap(s => Try(s.toInt).toOption), List("1", "x").map(s => Try(s.toInt).isSuccess))
