object L1:
  def trim(xs: java.util.ArrayList[String]): Int = { xs.trimToSize(); xs.size + 1 }
