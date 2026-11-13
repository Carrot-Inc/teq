object L2:
  def trim(xs: java.util.ArrayList[String]): Int = { xs.trimToSize(); xs.size + 2 }
