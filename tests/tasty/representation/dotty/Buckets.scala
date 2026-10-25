// Two effects of dotty's hash buckets that docs/internal/TYPES.md and DIFFERENCES.md declare and teq does not
// reproduce (the type-representation table's 3x-B SB1 and 3x-E VA6), shown on scalac 3.8.4's own classes: a record,
// never a case teq must match. Each set is dotty's own (`Uniques`, `NamedTypeUniques`) with its table made one bucket
// by reflection, so that "when the hashes share a bucket" holds by construction; everything else is dotty's code.
// Run by Oracle.scala, its lines pinned in buckets.txt.
package dotty.tools.dotc.core

import Contexts.*, Types.*, Symbols.*, Names.*, Flags.*, Scopes.*
import dotty.tools.dotc.util.WeakHashSet

object Buckets:
  /** The set's table replaced by one bucket and its threshold raised, so that every key lands in that bucket. */
  def oneBucket(set: WeakHashSet[?]): Unit =
    val cls = classOf[WeakHashSet[?]]
    val table = cls.getDeclaredField("table")
    table.setAccessible(true)
    table.set(set, java.lang.reflect.Array.newInstance(classOf[WeakHashSet.Entry[?]], 1))
    val threshold = cls.getDeclaredField("threshold")
    threshold.setAccessible(true)
    threshold.setInt(set, Int.MaxValue)

  def main(args: Array[String]): Unit =
    val base = new ContextBase
    val init = base.initialCtx.fresh
    init.setSetting(init.settings.Yusejavacp, true)
    given Context = init
    ctx.initialize()

    // 3x-B SB1: `RefinedType.eql` ignores precision (Types.scala 3306-3312); the hash's seed is the class
    // (Hashable.scala 45), so the two meet only in a shared bucket, where `WeakHashSet.put` (167-183) takes the
    // first entry `eql` to the key.
    val imprecise = RefinedType(defn.AnyRefType, termName("x"), defn.IntType)
    val precise = RefinedType.precise(defn.AnyRefType, termName("x"), defn.IntType)
    println("precise refinement is precise=" + precise.isPrecise + "; imprecise is precise=" + imprecise.isPrecise)
    println("eql ignores precision=" + imprecise.eql(precise))
    val uniques = new Uniques
    oneBucket(uniques)
    uniques.put(imprecise)
    println("in a shared bucket the precise refinement is the imprecise one=" + (uniques.put(precise) eq imprecise))

    // 3x-E VA6: `setDenot` rewrites a reference's symbolic designator in place (Types.scala 2622-2634), and
    // `NamedTypeUniques.enterIfNew` (Uniques.scala 41-67) compares an entry's live designator: in a shared bucket a
    // construction for the new symbol finds the rewritten reference.
    val cls = newCompleteClassSymbol(defn.EmptyPackageClass, typeName("BucketsProbe"), EmptyFlags, List(defn.AnyRefType), newScope)
    val old = newSymbol(cls, typeName("T"), EmptyFlags, TypeAlias(defn.IntType))
    cls.info.decls.openForMutations.enter(old)
    val named = new Uniques.NamedTypeUniques
    oneBucket(named)
    val ref = named.enterIfNew(cls.thisType, old, isTerm = false).asInstanceOf[TypeRef]
    val hashBefore = ref.hash
    cls.info.decls.openForMutations.unlink(old)
    val renewed = newSymbol(cls, typeName("T"), EmptyFlags, TypeAlias(defn.StringType))
    cls.info.decls.openForMutations.enter(renewed)
    ref.recomputeDenot()
    println("the designator rewritten in place=" + ((ref.designator: AnyRef) eq renewed) + "; its hash kept=" + (ref.hash == hashBefore))
    println("in a shared bucket a reference to the new symbol is the rewritten one=" +
      (named.enterIfNew(cls.thisType, renewed, isTerm = false) eq ref))
