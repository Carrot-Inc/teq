package sbt.internal.inc

import xsbti.api.DependencyContext

/** The dependency relations zinc stores in an `Analysis`, which its package keeps to itself:
  * per class the classes it depends on by each context, internal (a class of the same
  * compilation) and external (a class of an upstream analysis), the libraries of each source
  * with the one class zinc keeps per library, and the names each class uses. */
object OracleRelations:
  final case class Stored(
      classDeps: Seq[(String, String, String, String)],
      libraries: Seq[(String, String)],
      libraryClasses: Seq[(String, String)],
      names: Seq[(String, String, Seq[String])],
  )

  def of(analysis: Analysis, source: xsbti.VirtualFileRef => String, file: xsbti.VirtualFileRef => String): Stored =
    val r = analysis.relations
    def pairs(kind: String, context: DependencyContext, deps: Relations.ClassDependencies) =
      deps.internal.all.toSeq.map((from, to) => (from, kind, to, context.name)) ++
        deps.external.all.toSeq.map((from, to) => (from, "external", to, context.name))
    val classDeps =
      pairs("internal", DependencyContext.DependencyByMemberRef, r.memberRef) ++
        pairs("internal", DependencyContext.DependencyByInheritance, r.inheritance) ++
        pairs("internal", DependencyContext.LocalDependencyByInheritance, r.localInheritance) ++
        pairs("internal", DependencyContext.DependencyByMacroExpansion, r.macroExpansion)
    val names = r.names.iterator.toSeq.flatMap((c, used) => used.toSeq.map(u => (c, u.name, u.scopes.toArray.map(_.toString).toSeq.sorted)))
    Stored(
      classDeps.distinct.sorted,
      r.libraryDep.all.toSeq.map((s, l) => (source(s), file(l))).distinct.sorted,
      r.libraryClassName.all.toSeq.map((l, c) => (file(l), c)).distinct.sorted,
      names.sortBy(n => (n._1, n._2)),
    )
