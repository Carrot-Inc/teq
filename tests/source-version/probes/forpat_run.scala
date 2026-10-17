object P { def main(args: Array[String]): Unit = { val xs = List(Option(1), None); println(for (Some(x) <- xs) yield x) } }
