// The design pass's Identity probe (out/codex-probes/type-representation-design), run by Oracle.scala against
// scalac 3.8.4: dotty's identity rules the node/accessor table cites (docs/internal/TYPES.md), its lines pinned in
// identity.txt.
import dotty.tools.dotc.core.*
import Contexts.*, Types.*, Annotations.*, Constants.*, Names.*
import dotty.tools.dotc.ast.tpd
object Identity:
  def main(args: Array[String]): Unit =
    given Context = (new ContextBase).initialCtx
    val p = ConstantType(Constant(1))
    val a = ConcreteAnnotation(tpd.EmptyTree)
    val b = ConcreteAnnotation(tpd.EmptyTree)
    val x = AnnotatedType(p, a)
    val y = AnnotatedType(p, b)
    val same = AnnotatedType(p, a)
    println("annotation eql=" + a.eql(b))
    println("distinct annotation nodes eq=" + (x eq y))
    println("shared annotation nodes eq=" + (x eq same))
    println("bounds(p,p) kind=" + TypeBounds(p,p).getClass.getSimpleName)
    class ProbeMap extends ApproximatingTypeMap:
      def apply(t: Type): Type = t
      def probe(a: Type, b: Type): Type = atVariance(0)(range(a,b))
    val m = new ProbeMap
    println("range distinct=" + m.probe(x,y).getClass.getSimpleName)
    println("range shared=" + m.probe(x,same).getClass.getSimpleName)
    val s1 = SkolemType(p)
    val s2 = SkolemType(p)
    println("skolem eq=" + (s1 eq s2))
    var forces = 0
    val lazyAnn = new DeferredSymAndTree(Symbols.NoSymbol, { forces += 1; tpd.EmptyTree })
    println("deferred before=" + lazyAnn.isEvaluated)
    val sym = lazyAnn.symbol
    println("after symbol=" + lazyAnn.isEvaluated)
    val firstTree = lazyAnn.tree
    val secondTree = lazyAnn.tree
    println("after tree=" + lazyAnn.isEvaluated + "; forces=" + forces)
    println("derived same annotation=" + (lazyAnn.derivedAnnotation(firstTree) eq lazyAnn))
    val m1 = MethodType(List(termName("x")))(_ => List(p), _ => p)
    val m2 = MethodType(List(termName("x")))(_ => List(p), _ => p)
    println("method eq=" + (m1 eq m2))
    println("method unique eq=" + (Uniques.unique(m1) eq Uniques.unique(m2)))
