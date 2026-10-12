package rm

// The shape of reactivemongo's aggregation framework: an object nested in a trait with a self
// type, whose parents' arguments name the trait's instance (`pack.type`), a type alias sharing the
// object's name, operators nested in a generic trait with companions and private constructors.
trait SerializationPack:
  type Document
object BPack extends SerializationPack:
  type Document = String
trait PackSupport[P <: SerializationPack]:
  val pack: P
trait AggregationPipeline[P <: SerializationPack]:
  self: PackSupport[P] =>
  trait PipelineOperator
  final class Match private (val predicate: pack.Document) extends PipelineOperator:
    override def toString = s"Match($predicate)"
  object Match:
    def apply(predicate: pack.Document): Match = new Match(predicate)
  final class Limit private (val n: Int) extends PipelineOperator:
    override def toString = s"Limit($n)"
  object Limit:
    def apply(n: Int): Limit = new Limit(n)
trait AggFramework[P <: SerializationPack] extends AggregationPipeline[P]:
  self: PackSupport[P] =>
trait AggregationOps[P <: SerializationPack]:
  collection: GenericCollection[P] =>
  object AggregationFramework extends AggFramework[pack.type], PackSupport[pack.type]:
    val pack: collection.pack.type = collection.pack
  final type AggregationFramework = AggregationFramework.type
  final type PipelineOperator = AggregationFramework.PipelineOperator
trait GenericCollection[P <: SerializationPack] extends AggregationOps[P]:
  val pack: P
trait Meta
type Coll = GenericCollection[BPack.type] & Meta
class C extends GenericCollection[BPack.type], Meta:
  val pack: BPack.type = BPack
