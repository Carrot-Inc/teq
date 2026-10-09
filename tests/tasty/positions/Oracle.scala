//> using scala 3.8.4
//> using dep org.scala-lang::scala3-tasty-inspector:3.8.4

// What scalac 3.8.4's unpickler gives every tree of the TASTy files named on the command line:
// per tree in the order of a traversal, its class, its span
// (`none` without one, the point where the span has one) and its source's path. The traversal
// visits what `tpd.TreeTraverser` leaves out and the pickle holds: an `Inlined`'s call, an
// import's selectors, a `This`'s qualifier, a definition's annotations. A file the unpickler cannot
// read prints `# <file>: not read`. usage: Oracle.scala -- -cp=<class path> <file.tasty>...

import scala.quoted.*
import scala.tasty.inspector.*
import dotty.tools.dotc.ast.tpd
import dotty.tools.dotc.core.Contexts.Context

object Oracle:
  def main(args: Array[String]): Unit =
    val (cpArgs, files) = args.toList.partition(_.startsWith("-cp="))
    val cp = cpArgs.flatMap(_.stripPrefix("-cp=").split(':').toList.filter(_.nonEmpty))
    val inspector = new Inspector:
      def inspect(using Quotes)(tastys: List[Tasty[quotes.type]]): Unit =
        given ctx: Context = quotes.asInstanceOf[scala.quoted.runtime.impl.QuotesImpl].ctx
        for tasty <- tastys do
          println(s"# ${tasty.path}")
          val traverser = new tpd.TreeTraverser:
            def traverse(tree: tpd.Tree)(using Context): Unit =
              val s = tree.span
              if s.exists then
                val point = if s.isSynthetic then "" else s" point ${s.point}"
                println(s"${tree.getClass.getSimpleName} ${s.start} .. ${s.end}$point ${tree.source.path}")
              else println(s"${tree.getClass.getSimpleName} none")
              tree match
                case tpd.Inlined(call, _, _) if !call.isEmpty => traverse(call)
                case tpd.This(qual) if !qual.isEmpty => traverse(qual.asInstanceOf[tpd.Tree])
                case tree: tpd.ImportOrExport =>
                  for sel <- tree.selectors do
                    traverse(sel.imported)
                    if !sel.renamed.isEmpty then traverse(sel.renamed)
                    if !sel.bound.isEmpty then traverse(sel.bound)
                case tree: tpd.MemberDef if tree.symbol.exists =>
                  for annot <- tree.symbol.annotations do traverse(annot.tree)
                case _ =>
              traverseChildren(tree)
          traverser.traverse(tasty.ast.asInstanceOf[tpd.Tree])
    for file <- files do
      try
        if !TastyInspector.inspectAllTastyFiles(List(file), Nil, cp)(inspector) then println(s"# $file: not read")
      catch case e: Throwable => println(s"# $file: ${e.getClass.getSimpleName}")
