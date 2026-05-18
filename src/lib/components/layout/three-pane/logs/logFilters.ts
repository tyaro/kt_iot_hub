export type LogSource = 'grpc' | 'mqtt' | 'driver' | 'main' | 'registration-ui' | 'other';
export type Destination = 'lifecycle' | 'other';
export type ErrorFocus = 'all' | 'driver' | 'mqtt' | 'grpc';

export type ParsedLogLine = {
  raw: string;
  cleaned: string;
  timestamp: string;
  level: string;
  target: string;
  message: string;
  source: LogSource;
  lifecycle: boolean;
};

const ANSI_ESCAPE = String.fromCharCode(27);
const ANSI_ESCAPE_PATTERN = new RegExp(`${ANSI_ESCAPE}\\[[0-9;]*m`, 'g');

export function cleanLogText(raw: string): string {
  return raw
    .replace(ANSI_ESCAPE_PATTERN, '')
    .replace(/\uFFFD\[[0-9;]*m/g, '')
    .replace(/\s+/g, ' ')
    .trim();
}

export function parseLogLine(raw: string): ParsedLogLine {
  const cleaned = cleanLogText(raw);
  const levelMatch = cleaned.match(/\b(TRACE|DEBUG|INFO|WARN|ERROR)\b/);
  const level = levelMatch?.[1] ?? 'INFO';
  const structuredMatch = cleaned.match(
    /^(\d{4}-\d{2}-\d{2}T[^\s]+)\s+(TRACE|DEBUG|INFO|WARN|ERROR)\s+([a-zA-Z0-9_:-]+):\s*(.*)$/,
  );
  const fallbackMatch = cleaned.match(/\b(?:TRACE|DEBUG|INFO|WARN|ERROR)\s+([a-zA-Z0-9_:-]+):\s*(.*)$/);
  const timestamp = structuredMatch?.[1] ?? '-';
  const target = structuredMatch?.[3] ?? fallbackMatch?.[1] ?? 'unknown';
  const message = structuredMatch?.[4] ?? fallbackMatch?.[2] ?? cleaned;
  const lowerTarget = target.toLowerCase();
  const lowerMessage = message.toLowerCase();

  let source: LogSource = 'other';
  if (
    lowerTarget.includes('ui_launcher') ||
    lowerMessage.includes('driver ui process') ||
    lowerMessage.includes('launching driver ui')
  ) {
    source = 'registration-ui';
  } else if (lowerTarget.includes('grpc') || lowerMessage.includes('grpc')) {
    source = 'grpc';
  } else if (lowerTarget.includes('mqtt') || lowerMessage.includes('tagbus')) {
    source = 'mqtt';
  } else if (lowerTarget.includes('drivers') || lowerMessage.includes('driver process')) {
    source = 'driver';
  } else if (
    lowerMessage.includes('kt_iot_hub starting') ||
    lowerMessage.includes('graceful shutdown') ||
    lowerMessage.includes('exiting application') ||
    lowerTarget.includes('main')
  ) {
    source = 'main';
  }

  const lifecycle =
    /\b(start|starting|started|stop|stopping|stopped|launch|launched|shutdown|exiting|exited|terminate|terminated)\b/i.test(
      lowerMessage,
    ) || /\b(start|stop|launch|shutdown|exit)\b/i.test(lowerTarget);

  return {
    raw,
    cleaned,
    timestamp,
    level,
    target,
    message,
    source,
    lifecycle,
  };
}

export function filterLogLines(
  lines: ParsedLogLine[],
  options: {
    destination: Destination;
    lifecycleSource: LogSource | 'all';
    errorFocus: ErrorFocus;
    keyword: string;
  },
): ParsedLogLine[] {
  const { destination, lifecycleSource, errorFocus, keyword } = options;
  const kw = keyword.trim().toLowerCase();

  return lines.filter((line) => {
    const keywordMatched =
      kw.length === 0 ||
      line.cleaned.toLowerCase().includes(kw) ||
      line.message.toLowerCase().includes(kw) ||
      line.target.toLowerCase().includes(kw);
    if (!keywordMatched) {
      return false;
    }

    if (destination === 'lifecycle') {
      if (!line.lifecycle) {
        return false;
      }
      if (lifecycleSource !== 'all' && line.source !== lifecycleSource) {
        return false;
      }
      return true;
    }

    if (line.lifecycle) {
      return false;
    }

    if (errorFocus === 'all') {
      return true;
    }

    const isErrorLike = line.level === 'WARN' || line.level === 'ERROR';
    if (!isErrorLike) {
      return false;
    }

    if (errorFocus === 'driver') {
      return line.source === 'driver';
    }
    if (errorFocus === 'mqtt') {
      return line.source === 'mqtt';
    }
    if (errorFocus === 'grpc') {
      return line.source === 'grpc';
    }

    return true;
  });
}
