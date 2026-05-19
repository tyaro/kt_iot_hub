/* eslint-env browser */

/**
 * ステップウィザード共通フレームワーク
 *
 * 各ドライバ UI のステップ遷移ロジックを提供する。
 * ステップ固有の処理（レビュー描画等）は onBeforeStep コールバックで注入する。
 *
 * 依存: なし（globalThis.document を直接使用）
 */

/**
 * ステップを切り替える
 *
 * data-step-panel 属性を持つパネルの表示・非表示を切り替え、
 * .stepper .step ボタンの active / completed クラスを更新する。
 *
 * @param {number} step - 遷移先ステップ番号（1 始まり）
 * @param {(step: number) => void} [onBeforeStep] - ステップ切替前に呼ばれるコールバック
 * @param {Window & typeof globalThis} [win]
 */
export function setStep(step, onBeforeStep = null, win = globalThis) {
  win.document.querySelectorAll('[data-step-panel]').forEach((panel) => {
    panel.classList.toggle('hidden', Number(panel.dataset.stepPanel) !== step)
  })

  win.document.querySelectorAll('.stepper .step').forEach((button) => {
    const buttonStep = Number(button.dataset.step)
    button.classList.toggle('active', buttonStep === step)
    button.classList.toggle('completed', buttonStep < step)
  })

  if (onBeforeStep) {
    onBeforeStep(step)
  }
}

/**
 * .stepper .step ボタンに click イベントリスナを一括登録する
 *
 * @param {(targetStep: number) => void | Promise<void>} onStepSelect
 * @param {Window & typeof globalThis} [win]
 */
export function attachStepperEvents(onStepSelect, win = globalThis) {
  win.document.querySelectorAll('.stepper .step').forEach((button) => {
    button.addEventListener('click', async () => {
      const targetStep = Number(button.dataset.step)
      await onStepSelect(targetStep)
    })
  })
}
