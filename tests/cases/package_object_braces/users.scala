package apimodel.route

object Users:
  val list: String = ep("users".slash)
  def ids: List[Id] = List(Rank(2), Rank(1)).sorted.map(r => Id(r.n))
