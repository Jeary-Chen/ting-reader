export type MinuteMetric = {
  value: number;
  unit: 'minutes' | 'hours';
};

export const formatMinuteMetric = (minutes: number): MinuteMetric => {
  const safeMinutes = Number.isFinite(minutes)
    ? Math.max(0, Math.round(minutes))
    : 0;
  if (safeMinutes > 60) {
    return {
      value: Math.round(safeMinutes / 60),
      unit: 'hours',
    };
  }
  return { value: safeMinutes, unit: 'minutes' };
};
