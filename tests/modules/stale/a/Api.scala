package sta

class Store:
  def get(key: String): Option[Int] = if key.isEmpty then None else Some(key.length)
