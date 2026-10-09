package sbt.internal.teq

import java.io.File
import java.nio.file.Path
import java.util.{EnumSet, Optional}

import scala.collection.mutable

import xsbti.{Action, AnalysisCallback, AnalysisCallback3, DiagnosticCode, DiagnosticRelatedInformation, Position, Severity, T2, UseScope, VirtualFile, VirtualFileRef}
import xsbti.api.{ClassLike, DependencyContext}
import xsbti.compile.analysis.ReadSourceInfos

/** zinc's callback with the dependency calls a compile makes recorded as they arrive, each as
  * a canonical tuple: `usedName` (the class, the name, its scopes), `classDependency` (the
  * class depended on, the dependent class, the context) and `binaryDependency` (the binary
  * entry and the binary class name, the dependent class, its source, the context), paths and
  * sources through `path` and `source`, which name them apart from where the build put them;
  * and the products it reports, non-local (the source, the class file, the binary name) and
  * local (the source, the class file). Everything goes on to `delegate`. */
final class Recording(delegate: AnalysisCallback3, path: Path => String, source: VirtualFileRef => String) extends AnalysisCallback3:
  val usedNames = mutable.Set.empty[(String, String, Seq[String])]
  val classDependencies = mutable.Set.empty[(String, String, String)]
  val binaryDependencies = mutable.Set.empty[(String, String, String, String, String)]
  val products = mutable.Set.empty[(String, String, String)]
  val localProducts = mutable.Set.empty[(String, String)]

  def usedName(className: String, name: String, useScopes: EnumSet[UseScope]): Unit =
    synchronized { usedNames += ((className, name, useScopes.toArray.map(_.toString).toSeq.sorted)) }
    delegate.usedName(className, name, useScopes)

  def classDependency(onClassName: String, sourceClassName: String, context: DependencyContext): Unit =
    synchronized { classDependencies += ((onClassName, sourceClassName, context.name)) }
    delegate.classDependency(onClassName, sourceClassName, context)

  def binaryDependency(onBinaryEntry: Path, onBinaryClassName: String, fromClassName: String, fromSourceFile: VirtualFileRef, context: DependencyContext): Unit =
    synchronized { binaryDependencies += ((path(onBinaryEntry), onBinaryClassName, fromClassName, source(fromSourceFile), context.name)) }
    delegate.binaryDependency(onBinaryEntry, onBinaryClassName, fromClassName, fromSourceFile, context)

  @deprecated("", "") def binaryDependency(onBinaryEntry: File, onBinaryClassName: String, fromClassName: String, fromSourceFile: File, context: DependencyContext): Unit =
    binaryDependency(onBinaryEntry.toPath, onBinaryClassName, fromClassName, delegate.toVirtualFile(fromSourceFile.toPath), context)

  @deprecated("", "") def startSource(source: File): Unit = delegate.startSource(source)
  def startSource(source: VirtualFile): Unit = delegate.startSource(source)
  @deprecated("", "") def generatedNonLocalClass(source: File, classFile: File, binaryClassName: String, srcClassName: String): Unit =
    delegate.generatedNonLocalClass(source, classFile, binaryClassName, srcClassName)
  def generatedNonLocalClass(from: VirtualFileRef, classFile: Path, binaryClassName: String, srcClassName: String): Unit =
    synchronized { products += ((source(from), path(classFile), binaryClassName)) }
    delegate.generatedNonLocalClass(from, classFile, binaryClassName, srcClassName)
  @deprecated("", "") def generatedLocalClass(source: File, classFile: File): Unit = delegate.generatedLocalClass(source, classFile)
  def generatedLocalClass(from: VirtualFileRef, classFile: Path): Unit =
    synchronized { localProducts += ((source(from), path(classFile))) }
    delegate.generatedLocalClass(from, classFile)
  @deprecated("", "") def api(sourceFile: File, classApi: ClassLike): Unit = delegate.api(sourceFile, classApi)
  def api(sourceFile: VirtualFileRef, classApi: ClassLike): Unit = delegate.api(sourceFile, classApi)
  @deprecated("", "") def mainClass(sourceFile: File, className: String): Unit = delegate.mainClass(sourceFile, className)
  def mainClass(sourceFile: VirtualFileRef, className: String): Unit = delegate.mainClass(sourceFile, className)
  def problem(what: String, pos: Position, msg: String, severity: Severity, reported: Boolean): Unit = delegate.problem(what, pos, msg, severity, reported)
  def problem2(what: String, pos: Position, msg: String, severity: Severity, reported: Boolean, rendered: Optional[String], diagnosticCode: Optional[DiagnosticCode],
      diagnosticRelatedInformation: java.util.List[DiagnosticRelatedInformation], actions: java.util.List[Action]): Unit =
    delegate.problem2(what, pos, msg, severity, reported, rendered, diagnosticCode, diagnosticRelatedInformation, actions)
  def dependencyPhaseCompleted(): Unit = delegate.dependencyPhaseCompleted()
  def apiPhaseCompleted(): Unit = delegate.apiPhaseCompleted()
  def enabled(): Boolean = delegate.enabled()
  def classesInOutputJar(): java.util.Set[String] = delegate.classesInOutputJar()
  def isPickleJava(): Boolean = delegate.isPickleJava()
  def getPickleJarPair(): Optional[T2[Path, Path]] = delegate.getPickleJarPair()
  def toVirtualFile(path: Path): VirtualFile = delegate.toVirtualFile(path)
  def getSourceInfos(): ReadSourceInfos = delegate.getSourceInfos()

object Recording:
  def of(callback: AnalysisCallback, path: Path => String, source: VirtualFileRef => String): Recording = callback match
    case c: AnalysisCallback3 => new Recording(c, path, source)
    case other => throw new IllegalStateException(s"zinc's callback is a ${other.getClass.getName}, not an AnalysisCallback3")
