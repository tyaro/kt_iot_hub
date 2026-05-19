/* eslint-env browser */

/**
 * 全ドライバ UI 共通ユーティリティ
 *
 * 各ドライバの app.js から共通部分を抽出した共有モジュール。
 * DOM 操作・ログ出力・エラーフォーマットを提供する。
 * Tauri 固有機能は _shared/tauri.js を直接使用すること。
 */

export const UI_LOG_LIMIT = 300

/**
 * ログ詳細値を文字列に変換する（純粋関数）
 * @param {unknown} detail
 * @returns {string}
 */
export function formatLogDetail(detail) {
  if (detail == null) return ''
  if (typeof detail === 'string') return detail
  try {
    return JSON.stringify(detail)
  } catch {
    return String(detail)
  }
}

/**
 * UI ログパネルにログ行を追記する
 * @param {string} level - 'info' | 'warn' | 'error'
 * @param {string} message
 * @param {unknown} [detail]
 * @param {Window & typeof globalThis} [win]
 */
export function appendUiLog(level, message, detail = null, win = globalThis) {
  const list = win.document.getElementById('uiLogList')
  const time = new Date().toLocaleTimeString('ja-JP', { hour12: false })
  const line = `[${time}] [${level.toUpperCase()}] ${message}${detail == null ? '' : ` :: ${formatLogDetail(detail)}`}`

  if (list) {
    const row = win.document.createElement('div')
    row.className = `ui-log-row ${level}`
    row.textContent = line
    list.appendChild(row)

    while (list.childElementCount > UI_LOG_LIMIT) {
      list.removeChild(list.firstElementChild)
    }

    list.scrollTop = list.scrollHeight
  }

  if (level === 'error') {
    console.error(line)
  } else if (level === 'warn') {
    console.warn(line)
  } else {
    console.log(line)
  }
}

/**
 * UI ログパネルをクリアする
 * @param {Window & typeof globalThis} [win]
 */
export function clearUiLog(win = globalThis) {
  const list = win.document.getElementById('uiLogList')
  if (!list) return
  list.innerHTML = ''
  appendUiLog('info', 'UIログをクリアしました', null, win)
}

/**
 * ID で DOM 要素を取得するショートカット（globalThis.document を使用）
 * @param {string} id
 * @returns {HTMLElement | null}
 */
export const el = (id) => globalThis.document.getElementById(id)

/**
 * 複数の DOM 要素のテキストをクリアする
 * @param {Array<HTMLElement | null>} elements
 */
export function clearMessageElements(elements) {
  elements.forEach((element) => {
    if (element) {
      element.textContent = ''
    }
  })
}
