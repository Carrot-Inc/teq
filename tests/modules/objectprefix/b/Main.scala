import rm.*

extension (coll: Coll)
  def aggregateList(f: coll.AggregationFramework => (coll.PipelineOperator, List[coll.PipelineOperator])): String =
    f(coll.AggregationFramework).toString

@main def main(): Unit =
  val c: Coll = C()
  println(c.aggregateList: framework =>
    import framework.*
    Match("x") -> List(Limit(1)))
