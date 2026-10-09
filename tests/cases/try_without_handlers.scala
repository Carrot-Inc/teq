// A `try` without `catch` or `finally` is its body (zio-test-sbt's `SignalHandlers.install`
// is one): its value, its early return and its exception.
import scala.annotation.nowarn

@nowarn def install(): Unit = try {
  println("installed")
}

@nowarn def value: Int = try 42

@nowarn def early(x: Int): Int = try { if x > 0 then return x; -1 }

@nowarn def boom(): Int = try throw new IllegalStateException("boom")

@main def run(): Unit =
  install()
  println(value)
  println(early(3) + " " + early(-3))
  println(scala.util.Try(boom()).failed.get.getMessage)
