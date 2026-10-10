package sbt.internal.teq

import sbt.{Keys, RunningTaskEngine, Setting, State, TaskCancellationStrategy}

/** sbt's hook at the end of every evaluation of tasks, a cancelled one included: the strategy
  * of `taskCancelStrategy` (sbt's own) is told when the task engine starts and finishes. */
object Cancelling {
  def atEnd(action: () => Unit): Setting[?] =
    Keys.taskCancelStrategy ~= (inner => (state: State) => wrap(inner(state), action))

  private def wrap(strategy: TaskCancellationStrategy, action: () => Unit): TaskCancellationStrategy =
    new TaskCancellationStrategy {
      type State = strategy.State
      def onTaskEngineStart(canceller: RunningTaskEngine): State = strategy.onTaskEngineStart(canceller)
      def onTaskEngineFinish(state: State): Unit =
        try strategy.onTaskEngineFinish(state)
        finally action()
      override def toString: String = strategy.toString
    }
}
