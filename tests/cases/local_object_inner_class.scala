// A class of an object nested in a local object reads the local object's val: each call its own.
object Main { def main(args: Array[String]): Unit = {
  def run(n: Int): Int = {
    object O { val base = n; object I { class K { def get: Int = base } } }
    new O.I.K().get
  }
  println(run(21)); println(run(42))
}}
