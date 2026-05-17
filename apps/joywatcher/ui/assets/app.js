/* eslint-env browser */

const browserWindow = globalThis
const invoke = (cmd, args = {}) => browserWindow.__TAURI_INTERNALS__.invoke(cmd, args)

let launchContext = null
let scanGroups = []
let selectedGroupIndex = -1
let editingTagIndex = -1
let currentStep = 1
let isEditMode = false
let existingDriverIds = []
let browsedTags = []

const el = (id) => browserWindow.document.getElementById(id)

function formatError(error) {
  if (!error) return '不明なエラーが発生しました'
  if (typeof error === 'string') return error
  if (typeof error === 'object') {
    if (typeof error.error === 'string' && error.error) return error.error
    if (typeof error.message === 'string' && error.message) return error.message
    try {
      return JSON.stringify(error)
    } catch {
      return String(error)
    }
  }
  return String(error)
}

function clearMessages() {
  el('msgOk').textContent = ''
  el('msgErr').textContent = ''
  el('outOk').textContent = ''
  el('outErr').textContent = ''
}

function normalizeId(value) {
  return String(value || '')
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9_-]+/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '')
}

function connectionSettings() {
  return {
    endpoint: el('endpoint').value.trim(),
    user_id: Number(el('userId').value || 0),
    password: el('password').value,
    notes: el('notes').value.trim()
  }
}

function readConnectionSetting(settings, key, legacyKey, fallback = '') {
  if (!settings || typeof settings !== 'object') return fallback
  const value = settings[key] ?? settings[legacyKey]
  return value ?? fallback
}

function generateDefaultDriverId() {
  let num = 1
  while (existingDriverIds.includes(`joywatcher${num}`)) {
    num += 1
  }
  return `joywatcher${num}`
}

function restoreScanGroups(rawGroups) {
  if (!Array.isArray(rawGroups)) return []

  return rawGroups.map((group) => ({
    id: group.id || '',
    node: group.node || group.driverSpec?.node || '',
    scanRateMs: Number(group.scanRateMs || 1000),
    tags: Array.isArray(group.tags)
      ? group.tags.map((tag) => ({
          id: tag.id || '',
          name: tag.name || '',
          dataType: tag.dataType || 'f32',
          tagPath: tag.driverSpec?.tagPath || tag.driverSpec?.tag_path || '',
          nativeTagId:
            tag.driverSpec?.nativeTagId ??
            tag.driverSpec?.native_tag_id ??
            '',
          unit: tag.unit || tag.metadata?.unit || '',
          comment: tag.comment || tag.metadata?.comment || '',
          enabled: tag.enabled !== false
        }))
      : []
  }))
}

function activeGroup() {
  return scanGroups[selectedGroupIndex] || null
}

function totalTagCount() {
  return scanGroups.reduce((sum, group) => sum + group.tags.length, 0)
}

function deriveNameFromTagPath(tagPath) {
  const parts = String(tagPath || '').split('/').filter(Boolean)
  return parts[parts.length - 1] || tagPath
}

function setStep(step) {
  currentStep = step

  browserWindow.document.querySelectorAll('[data-step-panel]').forEach((panel) => {
    panel.classList.toggle('hidden', Number(panel.dataset.stepPanel) !== step)
  })

  browserWindow.document.querySelectorAll('.stepper .step').forEach((button) => {
    const buttonStep = Number(button.dataset.step)
    button.classList.toggle('active', buttonStep === step)
    button.classList.toggle('completed', buttonStep < step)
  })

  if (step === 3) {
    renderReview()
  }
}

function updateModeUi() {
  el('modeBadge').textContent = isEditMode ? '既存接続先の編集' : '新規登録'
  el('pageTitle').textContent = isEditMode ? 'JoyWatcher 接続先編集UI' : 'JoyWatcher 登録UI'
  el('pageDesc').textContent = isEditMode
    ? '既存接続先の設定を復元し、JoyWatcher 用の手動タグ構成を更新します。'
    : 'まずは手動定義で本体取込を確認し、DLL 接続は次段階で追加します。'
}

function refreshSummary() {
  const settings = connectionSettings()
  el('summary').innerHTML = `
    <div class="summary-card">
      <div class="summary-card-label">接続先ID</div>
      <div class="summary-card-value">${el('driverId').value.trim() || '-'}</div>
    </div>
    <div class="summary-card">
      <div class="summary-card-label">エンドポイント</div>
      <div class="summary-card-value">${settings.endpoint || '-'}</div>
    </div>
    <div class="summary-card">
      <div class="summary-card-label">User ID</div>
      <div class="summary-card-value">${settings.user_id}</div>
    </div>
    <div class="summary-card">
      <div class="summary-card-label">ScanGroup 数</div>
      <div class="summary-card-value">${scanGroups.length}</div>
    </div>
    <div class="summary-card">
      <div class="summary-card-label">タグ数</div>
      <div class="summary-card-value">${totalTagCount()}</div>
    </div>
    <div class="summary-card">
      <div class="summary-card-label">編集中グループ</div>
      <div class="summary-card-value">${activeGroup()?.id || '-'}</div>
    </div>
  `
}

function resetGroupForm() {
  el('groupId').value = ''
  el('groupNode').value = ''
  el('groupRate').value = '1000'
}

function resetTagForm() {
  editingTagIndex = -1
  el('tagId').value = ''
  el('tagName').value = ''
  el('tagDataType').value = 'f32'
  el('tagPath').value = ''
  el('tagNativeId').value = ''
  el('tagUnit').value = ''
  el('tagComment').value = ''
}

function selectGroup(index) {
  selectedGroupIndex = index
  editingTagIndex = -1
  const group = activeGroup()

  if (group) {
    el('groupId').value = group.id
    el('groupNode').value = group.node || ''
    el('groupRate').value = String(group.scanRateMs || 1000)
  } else {
    resetGroupForm()
  }

  resetTagForm()
  renderGroups()
  renderTags()
  refreshSummary()
}

function renderGroups() {
  el('groupsCountText').textContent = `${scanGroups.length}件`

  const list = el('groupsList')
  list.innerHTML = ''

  if (scanGroups.length === 0) {
    list.innerHTML = '<div class="list-item"><p class="muted">まだ ScanGroup がありません。</p></div>'
    el('activeGroupBadge').textContent = '未選択'
    return
  }

  scanGroups.forEach((group, index) => {
    const item = browserWindow.document.createElement('div')
    item.className = `list-item${index === selectedGroupIndex ? ' active' : ''}`
    item.innerHTML = `
      <div class="list-head">
        <h3>${group.id}</h3>
        <span class="badge ${index === selectedGroupIndex ? 'success' : 'neutral'}">タグ ${group.tags.length}件</span>
      </div>
      <div class="item-meta">
        <span>node: ${group.node || '-'}</span>
        <span>rate: ${group.scanRateMs} ms</span>
      </div>
      <div class="item-actions">
        <button type="button" class="btn secondary select-group">選択</button>
        <button type="button" class="btn ghost remove-group">削除</button>
      </div>
    `
    item.querySelector('.select-group').addEventListener('click', () => selectGroup(index))
    item.querySelector('.remove-group').addEventListener('click', () => {
      scanGroups.splice(index, 1)
      if (selectedGroupIndex >= scanGroups.length) {
        selectedGroupIndex = scanGroups.length - 1
      }
      if (selectedGroupIndex < 0) {
        resetGroupForm()
      }
      resetTagForm()
      renderGroups()
      renderTags()
      refreshSummary()
    })
    list.appendChild(item)
  })

  el('activeGroupBadge').textContent = activeGroup()?.id || '未選択'
}

function renderTags() {
  const list = el('tagsList')
  const group = activeGroup()
  list.innerHTML = ''

  if (!group) {
    list.innerHTML = '<div class="list-item"><p class="muted">先に ScanGroup を選択してください。</p></div>'
    return
  }

  if (group.tags.length === 0) {
    list.innerHTML = '<div class="list-item"><p class="muted">このグループにはまだタグがありません。</p></div>'
    return
  }

  group.tags.forEach((tag, index) => {
    const item = browserWindow.document.createElement('div')
    item.className = `list-item${index === editingTagIndex ? ' active' : ''}`
    item.innerHTML = `
      <div class="list-head">
        <h3>${tag.name}</h3>
        <span class="badge neutral">${tag.dataType}</span>
      </div>
      <div class="item-meta">
        <span>tagId: ${tag.id}</span>
        <span>tagPath: ${tag.tagPath}</span>
        <span>nativeTagId: ${tag.nativeTagId || '-'}</span>
      </div>
      <div class="item-actions">
        <button type="button" class="btn secondary edit-tag">編集</button>
        <button type="button" class="btn ghost remove-tag">削除</button>
      </div>
    `
    item.querySelector('.edit-tag').addEventListener('click', () => {
      editingTagIndex = index
      el('tagId').value = tag.id
      el('tagName').value = tag.name
      el('tagDataType').value = tag.dataType
      el('tagPath').value = tag.tagPath
      el('tagNativeId').value = tag.nativeTagId || ''
      el('tagUnit').value = tag.unit || ''
      el('tagComment').value = tag.comment || ''
      renderTags()
    })
    item.querySelector('.remove-tag').addEventListener('click', () => {
      group.tags.splice(index, 1)
      resetTagForm()
      renderGroups()
      renderTags()
      refreshSummary()
    })
    list.appendChild(item)
  })
}

function renderBrowsedTags() {
  const list = el('browsedTagsList')
  el('browsedTagsBadge').textContent = `${browsedTags.length}件`
  list.innerHTML = ''

  if (browsedTags.length === 0) {
    list.innerHTML = '<div class="list-item"><p class="muted">まだ参照結果がありません。</p></div>'
    return
  }

  browsedTags.forEach((tagPath) => {
    const item = browserWindow.document.createElement('div')
    item.className = 'list-item'
    item.innerHTML = `
      <div class="list-head">
        <h3>${deriveNameFromTagPath(tagPath)}</h3>
        <span class="badge neutral">選択候補</span>
      </div>
      <div class="item-meta">
        <span>${tagPath}</span>
      </div>
      <div class="item-actions">
        <button type="button" class="btn secondary pick-browsed-tag">このタグを使う</button>
      </div>
    `
    item.querySelector('.pick-browsed-tag').addEventListener('click', () => {
      el('tagPath').value = tagPath
      if (!el('tagName').value.trim()) {
        el('tagName').value = deriveNameFromTagPath(tagPath)
      }
      el('msgOk').textContent = `タグパスを反映しました: ${tagPath}`
    })
    list.appendChild(item)
  })
}

function validateConnection() {
  if (!el('driverId').value.trim()) {
    throw new Error('接続先IDを入力してください')
  }
}

async function resolveTagId() {
  validateConnection()

  const tagPath = el('tagPath').value.trim()
  if (!tagPath) {
    throw new Error('タグパスを入力してください')
  }

  const settings = connectionSettings()
  const nativeTagId = await invoke('resolve_joywatcher_tag', {
    endpoint: settings.endpoint,
    userId: settings.user_id,
    password: settings.password,
    tagPath
  })

  el('tagNativeId').value = String(nativeTagId)
  el('msgOk').textContent = `Tag ID を解決しました: ${nativeTagId}`
}

async function browseTags() {
  validateConnection()

  const settings = connectionSettings()
  const items = await invoke('browse_joywatcher_tags', {
    endpoint: settings.endpoint,
    userId: settings.user_id,
    password: settings.password
  })

  browsedTags = Array.isArray(items) ? items : []
  renderBrowsedTags()

  if (browsedTags.length === 1) {
    el('tagPath').value = browsedTags[0]
    if (!el('tagName').value.trim()) {
      el('tagName').value = deriveNameFromTagPath(browsedTags[0])
    }
  }

  el('msgOk').textContent = browsedTags.length > 0
    ? `${browsedTags.length} 件のタグ候補を取得しました`
    : 'タグ参照結果は空でした'
}

function upsertGroup() {
  const id = el('groupId').value.trim()
  const node = el('groupNode').value.trim()
  const scanRateMs = Number(el('groupRate').value || 1000)

  if (!id) throw new Error('ScanGroup ID を入力してください')
  if (!node) throw new Error('ノードを入力してください')
  if (scanRateMs < 100) throw new Error('読出し周期は100ms以上にしてください')

  const duplicateIndex = scanGroups.findIndex((group, index) => group.id === id && index !== selectedGroupIndex)
  if (duplicateIndex >= 0) {
    throw new Error(`同名 ScanGroup が既に存在します: ${id}`)
  }

  const nextGroup = {
    id,
    node,
    scanRateMs,
    tags: activeGroup()?.tags || []
  }

  if (selectedGroupIndex >= 0) {
    scanGroups.splice(selectedGroupIndex, 1, nextGroup)
  } else {
    scanGroups.push(nextGroup)
    selectedGroupIndex = scanGroups.length - 1
  }

  renderGroups()
  renderTags()
  refreshSummary()
  el('msgOk').textContent = `ScanGroup を保存しました: ${id}`
}

function upsertTag() {
  const group = activeGroup()
  if (!group) throw new Error('先に ScanGroup を選択してください')

  const name = el('tagName').value.trim()
  const tagPath = el('tagPath').value.trim()
  const nativeTagIdRaw = el('tagNativeId').value.trim()
  const explicitTagId = el('tagId').value.trim()
  const tagId = explicitTagId || normalizeId(`tag-${el('driverId').value}-${group.id}-${name || tagPath}`)

  if (!name) throw new Error('表示名を入力してください')
  if (!tagPath) throw new Error('タグパスを入力してください')
  if (!tagId) throw new Error('タグIDを入力してください')
  if (nativeTagIdRaw && !Number.isInteger(Number(nativeTagIdRaw))) {
    throw new Error('JoyWatcher Tag ID は整数で入力してください')
  }

  const duplicateIndex = group.tags.findIndex((tag, index) => tag.id === tagId && index !== editingTagIndex)
  if (duplicateIndex >= 0) {
    throw new Error(`同一タグIDが既に存在します: ${tagId}`)
  }

  const nextTag = {
    id: tagId,
    name,
    dataType: el('tagDataType').value,
    tagPath,
    nativeTagId: nativeTagIdRaw,
    unit: el('tagUnit').value.trim(),
    comment: el('tagComment').value.trim(),
    enabled: true
  }

  if (editingTagIndex >= 0) {
    group.tags.splice(editingTagIndex, 1, nextTag)
  } else {
    group.tags.push(nextTag)
  }

  resetTagForm()
  renderGroups()
  renderTags()
  refreshSummary()
  el('msgOk').textContent = `タグを保存しました: ${nextTag.name}`
}

function renderReview() {
  const settings = connectionSettings()
  el('reviewConnection').innerHTML = `
    <div class="review-card">
      <div class="summary-card-label">接続先ID</div>
      <div class="summary-card-value">${el('driverId').value.trim() || '-'}</div>
    </div>
    <div class="review-card">
      <div class="summary-card-label">エンドポイント</div>
      <div class="summary-card-value">${settings.endpoint || '-'}</div>
    </div>
    <div class="review-card">
      <div class="summary-card-label">User ID</div>
      <div class="summary-card-value">${settings.user_id}</div>
    </div>
  `
  el('reviewCounts').textContent = `ScanGroup ${scanGroups.length}件 / タグ ${totalTagCount()}件`

  const list = el('reviewGroups')
  list.innerHTML = ''

  if (scanGroups.length === 0) {
    list.innerHTML = '<div class="list-item"><p class="muted">保存対象の ScanGroup はありません。</p></div>'
    return
  }

  scanGroups.forEach((group) => {
    const item = browserWindow.document.createElement('div')
    item.className = 'list-item'
    item.innerHTML = `
      <div class="list-head">
        <h3>${group.id}</h3>
        <span class="badge success">タグ ${group.tags.length}件</span>
      </div>
      <div class="item-meta">
        <span>node: ${group.node}</span>
        <span>rate: ${group.scanRateMs} ms</span>
      </div>
      <div class="item-meta">
        <span>${group.tags.map((tag) => `${tag.name} (${tag.tagPath}${tag.nativeTagId ? ` / native:${tag.nativeTagId}` : ''})`).join(' / ') || '-'}</span>
      </div>
    `
    list.appendChild(item)
  })
}

function buildPayload() {
  validateConnection()

  return {
    schemaVersion: 1,
    driver: {
      id: el('driverId').value.trim(),
      driverType: 'joywatcher',
      enabled: true,
      settings: connectionSettings(),
      scanGroups: scanGroups.map((group) => ({
        id: group.id,
        node: group.node,
        scanRateMs: group.scanRateMs,
        tags: group.tags.map((tag) => ({
          id: tag.id,
          name: tag.name,
          dataType: tag.dataType,
          enabled: tag.enabled,
          unit: tag.unit || null,
          comment: tag.comment || null,
          driverSpec: {
            kind: 'joywatcher',
            node: group.node,
            scanGroup: group.id,
            tagPath: tag.tagPath,
            ...(tag.nativeTagId
              ? { nativeTagId: Number(tag.nativeTagId) }
              : {})
          }
        }))
      }))
    }
  }
}

async function saveOutput() {
  clearMessages()
  try {
    const payload = buildPayload()
    await invoke('save_driver_ui_output', {
      req: { outputJsonPath: launchContext?.outputJsonPath || null, payload }
    })

    el('outOk').textContent = '保存しました。ウィンドウを閉じます...'
    browserWindow.setTimeout(() => {
      void closeWindow()
    }, 200)
  } catch (error) {
    el('outErr').textContent = formatError(error)
  }
}

async function closeWindow() {
  try {
    await invoke('close_driver_ui_window')
    return
  } catch {
    browserWindow.close()
  }
}

async function init() {
  clearMessages()

  try {
    launchContext = await invoke('get_driver_ui_launch_context')
    const ctx = launchContext?.context || {}
    const settings = ctx.driverSettings && typeof ctx.driverSettings === 'object'
      ? ctx.driverSettings
      : {}

    isEditMode = Boolean(launchContext?.driverId)
    existingDriverIds = Array.isArray(ctx.existingDriverIds) ? ctx.existingDriverIds : []
    scanGroups = restoreScanGroups(ctx.scanGroups)

    el('driverId').value = launchContext?.driverId || generateDefaultDriverId()
    el('endpoint').value = readConnectionSetting(settings, 'endpoint', 'host', 'localhost')
    el('userId').value = String(readConnectionSetting(settings, 'user_id', 'userId', 0))
    el('password').value = readConnectionSetting(settings, 'password', 'passwd', '')
    el('notes').value = readConnectionSetting(settings, 'notes', 'memo', '')

    updateModeUi()
    renderGroups()
    renderTags()
    renderBrowsedTags()
    refreshSummary()
    if (scanGroups.length > 0) {
      selectGroup(0)
    }
    setStep(1)
  } catch (error) {
    el('msgErr').textContent = formatError(error)
  }
}

el('btnStep1Next').addEventListener('click', () => {
  clearMessages()
  try {
    validateConnection()
    setStep(2)
  } catch (error) {
    el('msgErr').textContent = formatError(error)
  }
})

el('btnStep2Prev').addEventListener('click', () => setStep(1))
el('btnStep2Next').addEventListener('click', () => setStep(3))
el('btnStep3Prev').addEventListener('click', () => setStep(2))
el('btnSaveGroup').addEventListener('click', () => {
  clearMessages()
  try {
    upsertGroup()
  } catch (error) {
    el('msgErr').textContent = formatError(error)
  }
})
el('btnResetGroup').addEventListener('click', () => {
  selectedGroupIndex = -1
  resetGroupForm()
  renderGroups()
  renderTags()
  refreshSummary()
})
el('btnResolveTag').addEventListener('click', async () => {
  clearMessages()
  try {
    await resolveTagId()
  } catch (error) {
    el('msgErr').textContent = formatError(error)
  }
})
el('btnBrowseTags').addEventListener('click', async () => {
  clearMessages()
  try {
    await browseTags()
  } catch (error) {
    el('msgErr').textContent = formatError(error)
  }
})
el('btnSaveTag').addEventListener('click', () => {
  clearMessages()
  try {
    upsertTag()
  } catch (error) {
    el('msgErr').textContent = formatError(error)
  }
})
el('btnResetTag').addEventListener('click', resetTagForm)
el('saveButton').addEventListener('click', () => void saveOutput())
el('cancelButton').addEventListener('click', () => void closeWindow())

;['driverId', 'endpoint', 'userId', 'notes'].forEach((id) => {
  el(id).addEventListener('input', refreshSummary)
})

browserWindow.document.querySelectorAll('.stepper .step').forEach((button) => {
  button.addEventListener('click', () => {
    const targetStep = Number(button.dataset.step)
    if (targetStep === 1) {
      setStep(1)
      return
    }
    if (targetStep === 2) {
      try {
        validateConnection()
        setStep(2)
      } catch (error) {
        el('msgErr').textContent = formatError(error)
      }
      return
    }
    setStep(3)
  })
})

void init()
