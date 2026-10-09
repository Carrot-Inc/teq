package sbt.internal.inc

import java.io.File
import java.util.Optional

import scala.collection.mutable
import scala.jdk.CollectionConverters.*

import sbt.internal.inc.caching.ClasspathCache
import sbt.util.Logger
import xsbti.{VirtualFile, VirtualFileRef}
import xsbti.compile.{APIChange as XAPIChange, CompileAnalysis, DefaultExternalHooks, ExternalHooks, FileHash, IncOptions, InitialChanges as XInitialChanges, InvalidationProfiler, RunProfiler, TastyFiles, TransactionalManagerType}

/** What zinc's own incremental run reports to the profiler sbt installs in its external hooks
  * (`xsbti.compile.InvalidationProfiler`), and the lookup sbt hands zinc, for the invalidation
  * oracle (`Invalidation`): nothing here invalidates, zinc's code does. */
object InvalidationZinc:
  /** An API change as zinc's invalidation sees it: the class, the kind (`NamesChange`,
    * `TraitPrivateMembersModified`, `APIChangeDueToMacroDefinition`,
    * `APIChangeDueToAnnotationDefinition`) and, for a `NamesChange`, the modified names with
    * their scopes. */
  final case class Change(cls: String, kind: String, names: Seq[(String, Seq[String])])
  final case class Initial(added: Seq[String], removed: Seq[String], changed: Seq[String], removedProducts: Seq[String], libraryDeps: Seq[String], external: Seq[Change])
  /** An invalidation event zinc's name hashing registers while it invalidates after a cycle: the
    * kind (inheritance, local inheritance, member reference, macro expansion), the classes it
    * starts from and those it invalidates. */
  final case class Event(kind: String, inputs: Seq[String], outputs: Seq[String])
  final case class Cycle(invalidated: Seq[String], packageObjects: Seq[String], initialSources: Seq[String], sources: Seq[String], recompiled: Seq[String],
      changes: Seq[Change], next: Seq[String], continues: Boolean, events: Seq[Event])

  def change(c: XAPIChange): Change = c match
    case NamesChange(cls, modified) =>
      Change(cls, "NamesChange", modified.names.toSeq.map(u => (u.name, u.scopes.asScala.toSeq.map(_.toString).sorted)).sorted)
    case TraitPrivateMembersModified(cls) => Change(cls, "TraitPrivateMembersModified", Nil)
    case APIChangeDueToMacroDefinition(cls) => Change(cls, "APIChangeDueToMacroDefinition", Nil)
    case APIChangeDueToAnnotationDefinition(cls) => Change(cls, "APIChangeDueToAnnotationDefinition", Nil)
    case other => Change(other.getModifiedClass, other.getClass.getSimpleName, Nil)

  /** The profiler of one `Incremental.apply`: the initial changes, then per cycle what zinc
    * registers, with the events registered since the previous cycle. Sources and products are
    * named through `source` and `product`. */
  final class Recorder(source: VirtualFileRef => String, product: VirtualFileRef => String) extends InvalidationProfiler:
    var initial: Option[Initial] = None
    val cycles = mutable.Buffer.empty[Cycle]
    private val pending = mutable.Buffer.empty[Event]

    def profileRun(): RunProfiler = new RunProfiler:
      def timeCompilation(startNanos: Long, durationNanos: Long): Unit = ()
      def registerInitial(c: XInitialChanges): Unit =
        val src = c.getInternalSrc
        def names(s: java.util.Set[VirtualFileRef]) = s.asScala.toSeq.map(source).sorted
        initial = Some(Initial(names(src.getAdded), names(src.getRemoved), names(src.getChanged), c.getRemovedProducts.asScala.toSeq.map(product).sorted,
          c.getLibraryDeps.asScala.toSeq.map(product).sorted, c.getExternal.toSeq.map(change).sortBy(_.cls)))
      def registerEvent(kind: String, inputs: Array[String], outputs: Array[String], reason: String): Unit =
        if outputs.nonEmpty then pending += Event(kind, inputs.toSeq.sorted, outputs.toSeq.sorted)
      def registerCycle(invalidated: Array[String], packageObjects: Array[String], initialSources: Array[VirtualFileRef], invalidatedSources: Array[VirtualFileRef],
          recompiled: Array[String], changes: Array[XAPIChange], next: Array[String], continues: Boolean): Unit =
        cycles += Cycle(invalidated.toSeq.sorted, packageObjects.toSeq.sorted, initialSources.toSeq.map(source).sorted, invalidatedSources.toSeq.map(source).sorted,
          recompiled.toSeq.sorted, changes.toSeq.map(change).sortBy(_.cls), next.toSeq.sorted, continues, pending.toSeq.distinct.sortBy(e => (e.kind, e.inputs.mkString(","))))
        pending.clear()
      def registerRun(): Unit = ()

  /** The options sbt 2.0.8 runs zinc with for a Scala 3 configuration (`Defaults.scala`: the
    * transactional class-file manager with its backup directory, `.tasty` files as auxiliary
    * class files, no external lookup), the profiler in the external hooks, and the scenario's
    * invalidation options. */
  def options(transitiveStep: Int, recompileAllFraction: Double, useOptimizedSealed: Boolean, profiler: InvalidationProfiler, backup: File, log: Logger): IncOptions =
    IncOptions.of()
      .withTransitiveStep(transitiveStep)
      .withRecompileAllFraction(recompileAllFraction)
      .withUseOptimizedSealed(useOptimizedSealed)
      .withStoreApis(true)
      .withAuxiliaryClassFiles(Array(TastyFiles.instance))
      .withClassfileManagerType(Optional.of(TransactionalManagerType.of(backup, log)))
      .withExternalHooks(new DefaultExternalHooks(Optional.empty(), Optional.empty(), ExternalHooks.NoProvenance.INSTANCE, profiler))

  /** The class path's hash as zinc's setup records it (`MixedAnalyzingCompiler.makeConfig`): a
    * directory's empty, a jar's by its content. */
  def classpathHash(classpath: Seq[VirtualFile], converter: xsbti.FileConverter): Vector[FileHash] =
    ClasspathCache.hashClasspath(classpath.map(converter.toPath)).toVector

  /** sbt 2.0.8's lookup: zinc's `LookupImpl`, sbt setting no external lookup in its hooks
    * (`Defaults.scala`, `externalHooks := IncOptions.defaultExternal`). The analyses are those of
    * the class path's entries, in its order; a binary class name's analysis is the first of them
    * whose products name it, and its analyzed class is that analysis's API of the class
    * (`Lookup.lookupAnalyzedClass`), whatever class file the class path holds; a class's entry on
    * the class path is the first that holds its class file (`Locate.entry`), which zinc asks for
    * a library's classes when the class path's hash changed. */
  final class SbtLookup(classpath: Seq[VirtualFile], analysisOf: VirtualFile => Option[Analysis], previousHash: Vector[FileHash], currentHash: Vector[FileHash])
      extends Lookup:
    lazy val analyses: Vector[CompileAnalysis] = classpath.toVector.flatMap(analysisOf)
    private lazy val entry = Locate.entry(classpath, new xsbti.compile.PerClasspathEntryLookup:
      def analysis(e: VirtualFile): Optional[CompileAnalysis] = analysisOf(e).fold(Optional.empty[CompileAnalysis])(Optional.of)
      def definesClass(e: VirtualFile): xsbti.compile.DefinesClass = Locate.definesClass(e)
    )
    def changedClasspathHash: Option[Vector[FileHash]] = if currentHash == previousHash then None else Some(currentHash)
    def lookupAnalysis(binaryClassName: String): Option[CompileAnalysis] =
      analyses.find { case a: Analysis => a.relations.productClassName._2s.contains(binaryClassName) }
    def lookupOnClasspath(binaryClassName: String): Option[VirtualFileRef] = entry(binaryClassName)
    def changedSources(previous: CompileAnalysis): Option[xsbti.compile.Changes[VirtualFileRef]] = None
    def changedBinaries(previous: CompileAnalysis): Option[Set[VirtualFileRef]] = None
    def removedProducts(previous: CompileAnalysis): Option[Set[VirtualFileRef]] = None
    def shouldDoIncrementalCompilation(changedClasses: Set[String], analysis: CompileAnalysis): Boolean = true
    override def hashClasspath(classpath: Array[VirtualFile]): Optional[Array[FileHash]] = Optional.empty()

  /** What an analysis says each source owns: its classes, its non-local products with their
    * binary names (a class file's, or a Scala.js check's stamp's), its local products; `product`
    * names a product `<entry>:<path>`, its path under the class directory after the colon. */
  final case class Ownership(classes: Seq[(String, String)], products: Seq[(String, String, String)], local: Seq[(String, String)])

  def ownership(a: Analysis, source: VirtualFileRef => String, product: VirtualFileRef => String): Ownership =
    val r = a.relations
    val binaries = r.productClassName._2s
    def binaryOf(p: VirtualFileRef): String =
      val s = product(p)
      s.substring(s.indexOf(':') + 1).stripSuffix(".class").stripSuffix(".teq").replace('/', '.')
    val classes = r.classes.all.toSeq.map((s, c) => (source(s), c)).sorted
    val all = r.srcProd.all.toSeq.map((s, p) => (source(s), p))
    val (nonLocal, local) = all.partition((_, p) => binaries.contains(binaryOf(p)))
    Ownership(classes, nonLocal.map((s, p) => (s, binaryOf(p), product(p))).sorted, local.map((s, p) => (s, product(p))).sorted)
