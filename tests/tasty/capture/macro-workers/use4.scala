object UseMacroWorkers4:
  def use(x: Any): Boolean = MacroWorkers.isString(x)
