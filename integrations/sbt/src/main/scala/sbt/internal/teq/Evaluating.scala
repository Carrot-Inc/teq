package sbt.internal.teq

import sbt.EvaluateTask

/** What sbt keeps to itself about the evaluation of tasks under way. */
object Evaluating:
  private object None

  /** The evaluation a task runs in: sbt has one at a time, and none between commands. */
  def now: AnyRef =
    val engine = EvaluateTask.currentlyRunningTaskEngine.get
    if engine == null then None else engine
