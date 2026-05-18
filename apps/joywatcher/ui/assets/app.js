/* eslint-env browser */

const browserWindow = globalThis
const tauriInvoke = browserWindow.__TAURI_INTERNALS__?.invoke?.bind(browserWindow.__TAURI_INTERNALS__)
const invoke = (cmd, args = {}) => {
  if (!tauriInvoke) {
    return Promise.reject(new Error('Tauri runtime is not available in static preview'))
  }
  return tauriInvoke(cmd, args)
}

let launchContext = null
let scanGroups = []
let selectedGroupIndex = -1
let isEditMode = false
let browsedTags = []
let isTypeProbeRunning = false
let connectionState = {
  endpoint: 'localhost',
  user_id: 1,
  password: '',
  notes: ''
}

const el = (id) => browserWindow.document.getElementById(id)

function setTypeProbeBusy(busy) {
  isTypeProbeRunning = busy
  const button = el('btnProbeTypes')
  if (!button) return
  button.disabled = busy
  button.textContent = busy ? '型確認中...' : '型確認 (JWRead)'
}

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
  if (el('outOkReview')) el('outOkReview').textContent = ''
  if (el('outErrReview')) el('outErrReview').textContent = ''
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
  return { ...connectionState }
}

function hasImportedTags() {
  return totalTagCount() > 0
}

function countDetectedTypes() {
  return scanGroups.reduce(
    (acc, group) => {
      group.tags.forEach((tag) => {
        if (!tag.detectedValueKind) return
        if (tag.detectedValueKind === 'bool' || tag.detectedValueKind === 'string' || tag.detectedValueKind === 'number') {
          acc[tag.detectedValueKind] += 1
          acc.confirmed += 1
        }
      })
      return acc
    },
    { bool: 0, string: 0, number: 0, confirmed: 0 }
  )
}

function countResolvedNativeTagIds() {
  return scanGroups.reduce(
    (sum, group) => sum + group.tags.filter((tag) => Number.isInteger(Number(tag.nativeTagId)) && Number(tag.nativeTagId) > 0).length,
    0
  )
}

function currentResolvedConnectionId() {
  return syncDriverIdFromGroups() || launchContext?.driverId || ''
}

function collectConnectionIds() {
  const ids = new Set()

  browsedTags.forEach((item) => {
    if (item?.connectionId) ids.add(item.connectionId)
  })

  scanGroups.forEach((group) => {
    if (group?.node) ids.add(group.node)
  })

  return Array.from(ids)
}

function listTagLabels(predicate) {
  return scanGroups.flatMap((group) =>
    group.tags
      .filter(predicate)
      .map((tag) => `${group.id} / ${tag.name}`)
  )
}

function createReviewAlert({ key, title, severity, statusLabel, summary, count, details = [] }) {
  return {
    key,
    title,
    severity,
    statusLabel,
    summary,
    count,
    details
  }
}

function collectReviewAlerts() {
  const totalTags = totalTagCount()
  const resolvedTagIds = countResolvedNativeTagIds()
  const detected = countDetectedTypes()
  const connectionIds = collectConnectionIds()
  const unresolvedIdLabels = listTagLabels((tag) => !(Number.isInteger(Number(tag.nativeTagId)) && Number(tag.nativeTagId) > 0))
  const unresolvedTypeLabels = listTagLabels((tag) => !tag.detectedValueKind)
  const resolvedConnectionId = currentResolvedConnectionId()

  const connectionAlert = connectionIds.length > 1
    ? createReviewAlert({
        key: 'connection-mixed',
        title: '接続先混在',
        severity: 'danger',
        statusLabel: '要確認',
        summary: `複数の接続先IDが混在しています (${connectionIds.join(', ')})。`,
        count: connectionIds.length,
        details: [
          'SelTag2 の取込結果が複数接続先にまたがっています。',
          '別接続先のタグが混ざっていないか確認してください。'
        ]
      })
    : !resolvedConnectionId
      ? createReviewAlert({
          key: 'connection-mixed',
          title: '接続先混在',
          severity: 'warn',
          statusLabel: '未確定',
          summary: 'SelTag2 由来の接続先IDがまだ確定していません。',
          count: 0,
          details: ['SelTag2 を実行して接続先IDを確定してください。']
        })
      : createReviewAlert({
          key: 'connection-mixed',
          title: '接続先混在',
          severity: 'ok',
          statusLabel: '問題なし',
          summary: `接続先IDは ${resolvedConnectionId} で統一されています。`,
          count: 1,
          details: connectionIds.length > 0 ? [`接続先候補: ${connectionIds.join(', ')}`] : []
        })

  const unresolvedIdCount = Math.max(totalTags - resolvedTagIds, 0)
  const unresolvedIdAlert = unresolvedIdCount > 0
    ? createReviewAlert({
        key: 'unresolved-id',
        title: 'ID未解決',
        severity: 'danger',
        statusLabel: '要確認',
        summary: `nativeTagId が未解決のタグがあります (${unresolvedIdCount}件)。`,
        count: unresolvedIdCount,
        details: unresolvedIdLabels.slice(0, 5)
      })
    : createReviewAlert({
        key: 'unresolved-id',
        title: 'ID未解決',
        severity: 'ok',
        statusLabel: '問題なし',
        summary: `nativeTagId は ${resolvedTagIds} / ${totalTags} 件解決済みです。`,
        count: 0,
        details: []
      })

  const unresolvedTypeCount = Math.max(totalTags - detected.confirmed, 0)
  const unresolvedTypeAlert = unresolvedTypeCount > 0
    ? createReviewAlert({
        key: 'unconfirmed-type',
        title: '型未確認',
        severity: 'warn',
        statusLabel: '要確認',
        summary: `型確認が未実施のタグがあります (${unresolvedTypeCount}件)。`,
        count: unresolvedTypeCount,
        details: unresolvedTypeLabels.slice(0, 5)
      })
    : createReviewAlert({
        key: 'unconfirmed-type',
        title: '型未確認',
        severity: 'ok',
        statusLabel: '問題なし',
        summary: `型確認は ${detected.confirmed} / ${totalTags} 件完了しています。`,
        count: 0,
        details: []
      })

  return [connectionAlert, unresolvedIdAlert, unresolvedTypeAlert]
}

function readConnectionSetting(settings, key, legacyKey, fallback = '') {
  if (!settings || typeof settings !== 'object') return fallback
  const value = settings[key] ?? settings[legacyKey]
  return value ?? fallback
}

function currentDriverId() {
  return el('driverId')?.value.trim() || activeGroup()?.node || scanGroups[0]?.node || launchContext?.driverId || ''
}

function syncDriverIdFromGroups() {
  const nextDriverId = activeGroup()?.node || scanGroups[0]?.node || launchContext?.driverId || el('driverId')?.value.trim() || ''
  if (el('driverId')) {
    el('driverId').value = nextDriverId
  }
  return nextDriverId
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
          detectedValueKind: tag.driverSpec?.detectedValueKind || tag.driverSpec?.detected_value_kind || '',
          detectedDtype:
            tag.driverSpec?.detectedDtype ??
            tag.driverSpec?.detected_dtype ??
            null,
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

function summarizeGroupQuality(group) {
  const total = group?.tags?.length || 0
  const detected = (group?.tags || []).filter((tag) => Boolean(tag.detectedValueKind)).length
  const resolved = (group?.tags || []).filter((tag) => Number.isInteger(Number(tag.nativeTagId)) && Number(tag.nativeTagId) > 0).length
  const resolvedConnectionId = currentResolvedConnectionId()

  let connectionTone = 'neutral'
  let connectionLabel = '接続 未確認'

  if (group?.node) {
    if (!resolvedConnectionId) {
      connectionTone = 'warn'
      connectionLabel = `接続 ${group.node}`
    } else if (group.node !== resolvedConnectionId) {
      connectionTone = 'danger'
      connectionLabel = `接続差異 ${group.node}`
    } else {
      connectionTone = 'ok'
      connectionLabel = `接続 ${group.node}`
    }
  }

  return {
    total,
    detected,
    resolved,
    connectionTone,
    connectionLabel,
    typeTone: detected < total ? 'warn' : 'ok',
    idTone: resolved < total ? 'danger' : 'ok'
  }
}

function parseJoyWatcherTagPath(tagPath) {
  const raw = String(tagPath || '').trim()
  if (!raw) return null

  const match = raw.match(/^([^$]+)\$([^$]+)\$VALUE$/i)
  if (!match) return null

  const connectionId = match[1]
  const body = match[2]
  const dotIndex = body.lastIndexOf('.')

  if (dotIndex < 0) {
    return {
      raw,
      connectionId,
      groupId: body,
      tagName: body
    }
  }

  return {
    raw,
    connectionId,
    groupId: body.slice(0, dotIndex),
    tagName: body.slice(dotIndex + 1)
  }
}

function parseResolvedBrowseItem(value) {
  const raw = String(value || '').trim()
  if (!raw) return null

  const separatorIndex = raw.lastIndexOf('|')
  if (separatorIndex < 0) return null

  const tagPath = raw.slice(0, separatorIndex)
  const nativeTagId = raw.slice(separatorIndex + 1)
  const parsed = parseJoyWatcherTagPath(tagPath)
  if (!parsed) return null

  return {
    ...parsed,
    nativeTagId
  }
}

function groupKey(groupId, node) {
  return `${node}::${groupId}`
}

function findGroupIndex(groupId, node) {
  return scanGroups.findIndex((group) => group.id === groupId && group.node === node)
}

function buildAutoTag(driverId, parsedTag) {
  return {
    id: normalizeId(`tag-${driverId}-${parsedTag.connectionId}-${parsedTag.groupId}-${parsedTag.tagName}`),
    name: parsedTag.tagName,
    dataType: 'f32',
    tagPath: parsedTag.raw,
    nativeTagId: parsedTag.nativeTagId,
    detectedValueKind: '',
    detectedDtype: null,
    unit: '',
    comment: '',
    enabled: true
  }
}

function mapJoyWatcherDtypeToDataType(dtype, fallback = 'f32') {
  if (!Number.isInteger(dtype)) return fallback

  switch (dtype) {
    case 0:
      return 'i32'
    case 1:
      return 'i64'
    case 2:
      return 'f32'
    case 3:
      return 'f64'
    case 6:
      return 'i32'
    case 7:
      return 'i64'
    default:
      return fallback
  }
}

function mapDetectedValueKindToDataType(valueKind, dtype, fallback = 'f32') {
  if (valueKind === 'bool') return 'bool'
  if (valueKind === 'string') return 'string'
  if (valueKind === 'number') {
    return mapJoyWatcherDtypeToDataType(dtype, fallback)
  }
  return fallback
}

function displayTagType(tag) {
  if (tag?.detectedValueKind === 'bool') return 'bool'
  if (tag?.detectedValueKind === 'string') return 'string'
  if (tag?.detectedValueKind === 'number') return tag?.dataType || 'f32'
  return '未設定'
}

function tagTypeBadgeClass(tag) {
  const type = displayTagType(tag).toLowerCase()
  if (type === 'bool') return 'tag-type-badge bool'
  if (type === 'string') return 'tag-type-badge string'
  if (type === 'i32') return 'tag-type-badge i32'
  if (type === 'i64') return 'tag-type-badge i64'
  if (type === 'f32') return 'tag-type-badge f32'
  if (type === 'f64') return 'tag-type-badge f64'
  if (type === '未設定') return 'tag-type-badge neutral'
  return 'tag-type-badge neutral'
}

function updateDetectedTagTypes(items) {
  if (!Array.isArray(items) || items.length === 0) {
    return { updatedCount: 0, counts: { bool: 0, string: 0, number: 0 } }
  }

  const parsedItems = items
    .map((item) => {
      if (typeof item !== 'string') return null
      const [tagIdText, valueKind = '', quality = '', dtypeText = ''] = item.split('|')
      const tagId = Number(tagIdText)
      if (!Number.isFinite(tagId) || !valueKind) return null
      const parsedDtype = Number(dtypeText)
      return {
        tagId,
        valueKind,
        quality,
        dtype: Number.isInteger(parsedDtype) ? parsedDtype : null
      }
    })
    .filter(Boolean)

  const counts = { bool: 0, string: 0, number: 0 }
  const typeMap = new Map(
    parsedItems
      .filter((item) => Number.isFinite(Number(item?.tagId)) && typeof item?.valueKind === 'string')
      .map((item) => [Number(item.tagId), { valueKind: item.valueKind, dtype: item.dtype }])
  )

  let updatedCount = 0
  scanGroups.forEach((group) => {
    group.tags.forEach((tag) => {
      const nativeTagId = Number(tag.nativeTagId)
      const detectedType = typeMap.get(nativeTagId)
      if (!detectedType) return

      const valueKind = detectedType.valueKind
      tag.detectedValueKind = valueKind
      tag.detectedDtype = Number.isInteger(detectedType.dtype) ? detectedType.dtype : null
      tag.dataType = mapDetectedValueKindToDataType(valueKind, tag.detectedDtype, tag.dataType || 'f32')
      updatedCount += 1

      if (valueKind === 'bool' || valueKind === 'string' || valueKind === 'number') {
        counts[valueKind] += 1
      }
    })
  })

  return { updatedCount, counts }
}

function refreshTablesAfterTypeProbe() {
  renderGroups()
  renderTags()
  refreshSummary()
}

async function probeImportedTagTypes(parsedItems) {
  const settings = connectionSettings()
  const tagIds = Array.from(
    new Set(
      (Array.isArray(parsedItems) ? parsedItems : [])
        .map((item) => Number(item?.nativeTagId))
        .filter((tagId) => Number.isInteger(tagId) && tagId > 0)
    )
  )

  if (tagIds.length === 0) {
    return { updatedCount: 0, counts: { bool: 0, string: 0, number: 0 } }
  }

  const detected = await invoke('probe_joywatcher_tag_types', {
    endpoint: settings.endpoint,
    userId: settings.user_id,
    password: settings.password,
    tagIds
  })

  const summary = updateDetectedTagTypes(Array.isArray(detected) ? detected : [])
  if (summary.updatedCount > 0) {
    refreshTablesAfterTypeProbe()
  }
  return summary
}

function collectRegisteredNativeTagIds() {
  return Array.from(
    new Set(
      scanGroups.flatMap((group) =>
        group.tags
          .map((tag) => Number(tag.nativeTagId))
          .filter((tagId) => Number.isInteger(tagId) && tagId > 0)
      )
    )
  )
}

async function probeRegisteredTagTypes() {
  if (isTypeProbeRunning) {
    return { updatedCount: 0, counts: { bool: 0, string: 0, number: 0 } }
  }

  validateConnection()

  const settings = connectionSettings()
  const tagIds = collectRegisteredNativeTagIds()
  if (tagIds.length === 0) {
    throw new Error('型確認対象のタグがありません。先に TagSel2 でタグを取り込んでください')
  }

  const detected = await invoke('probe_joywatcher_tag_types', {
    endpoint: settings.endpoint,
    userId: settings.user_id,
    password: settings.password,
    tagIds
  })

  const summary = updateDetectedTagTypes(Array.isArray(detected) ? detected : [])
  refreshTablesAfterTypeProbe()
  return summary
}

function importBrowsedTags(parsedItems) {
  const importedGroupKeys = []
  const driverId = parsedItems[0]?.connectionId || currentDriverId()

  parsedItems.forEach((item) => {
    const key = groupKey(item.groupId, item.connectionId)
    let targetIndex = findGroupIndex(item.groupId, item.connectionId)
    if (targetIndex < 0) {
      scanGroups.push({
        id: item.groupId,
        node: item.connectionId,
        scanRateMs: 1000,
        tags: []
      })
      targetIndex = scanGroups.length - 1
    }

    const group = scanGroups[targetIndex]
    const nextTag = buildAutoTag(driverId, item)
    const existingIndex = group.tags.findIndex((tag) => tag.tagPath === nextTag.tagPath)

    if (existingIndex >= 0) {
      group.tags.splice(existingIndex, 1, {
        ...group.tags[existingIndex],
        id: group.tags[existingIndex].id || nextTag.id,
        name: nextTag.name,
        tagPath: nextTag.tagPath,
        nativeTagId: nextTag.nativeTagId
      })
    } else {
      group.tags.push(nextTag)
    }

    if (!importedGroupKeys.includes(key)) {
      importedGroupKeys.push(key)
    }
  })

  if (importedGroupKeys.length > 0) {
    syncDriverIdFromGroups()
    const firstIndex = scanGroups.findIndex((group) => groupKey(group.id, group.node) === importedGroupKeys[0])
    if (firstIndex >= 0) {
      selectGroup(firstIndex)
      return
    }
  }

  renderGroups()
  renderTags()
  refreshSummary()
}

function setStep(step) {
  browserWindow.document.querySelectorAll('[data-step-panel]').forEach((panel) => {
    panel.classList.toggle('hidden', Number(panel.dataset.stepPanel) !== step)
  })

  browserWindow.document.querySelectorAll('.stepper .step').forEach((button) => {
    const buttonStep = Number(button.dataset.step)
    button.classList.toggle('active', buttonStep === step)
    button.classList.toggle('completed', buttonStep < step)
  })

  if (step === 2) {
    renderReview()
  }
}

function updateModeUi() {
  el('modeBadge').textContent = isEditMode ? '既存接続先の編集' : '新規登録'
  el('pageTitle').textContent = isEditMode ? 'JoyWatcher 接続先編集UI' : 'JoyWatcher 登録UI'
  el('pageDesc').textContent = isEditMode
    ? '既存定義を復元し、タグ登録ボタンから TagSel2 を繰り返し実行してグループ単位で更新します。'
    : 'タグ登録ボタンから TagSel2 を実行し、選択結果から接続先IDとタグ定義を自動登録します。'
}

function refreshSummary() {
  const driverId = syncDriverIdFromGroups()
  const detected = countDetectedTypes()
  const connectionIds = collectConnectionIds()
  el('summary').innerHTML = `
    <div class="summary-card">
      <div class="summary-card-label">接続先ID</div>
      <div class="summary-card-value">${driverId || '-'}</div>
    </div>
    <div class="summary-card">
      <div class="summary-card-label">SelTag2 接続先候補</div>
      <div class="summary-card-value">${connectionIds.length || 0}</div>
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
    <div class="summary-card">
      <div class="summary-card-label">型確認済み</div>
      <div class="summary-card-value">${detected.confirmed} / ${totalTagCount()}</div>
    </div>
  `

  renderImportStatus()
}

function renderImportStatus() {
  const summaryEl = el('importSummaryText')
  const groupBadgeEl = el('activeGroupBadge')
  const tagCountEl = el('tagTableCountText')
  const group = activeGroup()

  if (summaryEl) {
    if (browsedTags.length > 0) {
      summaryEl.textContent = `今回 ${browsedTags.length} 件、累計 ${scanGroups.length} グループ / ${totalTagCount()} タグを取り込み済みです`
    } else if (totalTagCount() > 0) {
      summaryEl.textContent = `累計 ${scanGroups.length} グループ / ${totalTagCount()} タグが登録されています`
    } else {
      summaryEl.textContent = 'まだタグは取り込まれていません'
    }
  }

  if (groupBadgeEl) {
    groupBadgeEl.textContent = group ? group.id : '未選択'
  }

  if (tagCountEl) {
    tagCountEl.textContent = `${group?.tags.length || 0}件`
  }
}

function resetGroupForm() {
  el('groupId').value = ''
  el('groupRate').value = '1000'
}

function resetTagForm() {
  renderImportStatus()
}

function removeGroup(index) {
  scanGroups.splice(index, 1)
  if (selectedGroupIndex === index) {
    selectedGroupIndex = scanGroups.length > 0 ? Math.min(index, scanGroups.length - 1) : -1
  } else if (selectedGroupIndex > index) {
    selectedGroupIndex -= 1
  }

  if (selectedGroupIndex >= 0) {
    const group = activeGroup()
    el('groupId').value = group?.id || ''
    el('groupRate').value = String(group?.scanRateMs || 1000)
  } else {
    resetGroupForm()
  }

  renderGroups()
  renderTags()
  refreshSummary()
}

function removeTag(groupIndex, tagIndex) {
  const group = scanGroups[groupIndex]
  if (!group) return
  group.tags.splice(tagIndex, 1)
  renderGroups()
  renderTags()
  refreshSummary()
}

function selectGroup(index) {
  selectedGroupIndex = index
  const group = activeGroup()

  if (group) {
    el('groupId').value = group.id
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
  el('groupsCountText').textContent = `登録済み ${scanGroups.length}件`

  const list = el('groupsList')
  const hint = el('groupsHint')
  list.innerHTML = ''

  if (scanGroups.length === 0) {
    if (hint) {
      hint.textContent = 'TagSel2 でタグを取り込むと、ここにグループが並びます。'
      hint.style.display = 'block'
    }
    el('activeGroupBadge').textContent = '未選択'
    renderImportStatus()
    return
  }

  if (hint) {
    hint.style.display = 'none'
  }

  scanGroups.forEach((group, index) => {
    const quality = summarizeGroupQuality(group)
    const item = browserWindow.document.createElement('div')
    item.className = `jw-group-row${index === selectedGroupIndex ? ' active' : ''}`
    item.innerHTML = `
      <span class="jw-table-cell name group-name-cell">
        <span class="group-name-main">${group.id}</span>
      </span>
      <span class="jw-table-cell group-status-cell">
        <span class="group-quality-line">
          <span class="group-inline-badge ${quality.connectionTone}">${quality.connectionLabel}</span>
          <span class="group-inline-badge ${quality.typeTone}">型 ${quality.detected}/${quality.total}</span>
          <span class="group-inline-badge ${quality.idTone}">ID ${quality.resolved}/${quality.total}</span>
        </span>
      </span>
      <span class="jw-table-cell center">${group.scanRateMs} ms</span>
      <span class="jw-table-cell center">${group.tags.length}</span>
      <div class="jw-row-actions">
        <button type="button" class="btn ghost select-group">選択</button>
        <button type="button" class="btn ghost remove-group">削除</button>
      </div>
    `
    item.addEventListener('click', () => selectGroup(index))
    item.querySelectorAll('.select-group').forEach((button) => {
      button.addEventListener('click', (event) => {
        event.stopPropagation()
        selectGroup(index)
      })
    })
    item.querySelector('.remove-group').addEventListener('click', (event) => {
      event.stopPropagation()
      removeGroup(index)
    })
    list.appendChild(item)
  })

  renderImportStatus()
}

function renderTags() {
  const list = el('tagsList')
  const hint = el('tagsHint')
  const group = activeGroup()
  list.innerHTML = ''

  if (!group) {
    if (hint) {
      hint.textContent = 'グループを選択すると、タグ一覧が表示されます。'
      hint.style.display = 'block'
    }
    renderImportStatus()
    return
  }

  if (group.tags.length === 0) {
    if (hint) {
      hint.textContent = 'このグループにはまだタグがありません。TagSel2 で追加取り込みしてください。'
      hint.style.display = 'block'
    }
    renderImportStatus()
    return
  }

  if (hint) {
    hint.style.display = 'none'
  }

  group.tags.forEach((tag, index) => {
    const item = browserWindow.document.createElement('div')
    item.className = 'jw-tag-row'
    item.innerHTML = `
      <span class="jw-table-cell name">${tag.name}</span>
      <span class="jw-table-cell center"><span class="badge ${tagTypeBadgeClass(tag)}">${displayTagType(tag)}</span></span>
      <span class="jw-table-cell center">${tag.nativeTagId || '-'}</span>
      <div class="jw-row-actions">
        <button type="button" class="btn ghost remove-tag">削除</button>
      </div>
    `
    item.querySelector('.remove-tag').addEventListener('click', () => removeTag(selectedGroupIndex, index))
    list.appendChild(item)
  })
}

function renderBrowsedTags() {
  const list = el('browsedTagsList')
  if (!list) return
  el('browsedTagsBadge').textContent = `${browsedTags.length}件`
  list.innerHTML = ''

  if (browsedTags.length === 0) {
    list.innerHTML = '<div class="list-item"><p class="muted">まだ今回の取込結果はありません。`タグ登録 (TagSel2)` を押してください。</p></div>'
    return
  }

  const groups = new Map()
  browsedTags.forEach((item) => {
    const key = groupKey(item.groupId, item.connectionId)
    if (!groups.has(key)) {
      groups.set(key, {
        connectionId: item.connectionId,
        groupId: item.groupId,
        items: []
      })
    }
    groups.get(key).items.push(item)
  })

  Array.from(groups.values()).forEach((group) => {
    const item = browserWindow.document.createElement('div')
    item.className = 'list-item'
    item.innerHTML = `
      <div class="list-head">
        <h3>${group.groupId}</h3>
        <span class="badge neutral">${group.connectionId}</span>
      </div>
      <div class="item-meta">
        <span>${group.items.length}件のタグ</span>
      </div>
      <div class="item-meta">
        <span>${group.items.map((entry) => `${entry.tagName}${entry.nativeTagId ? ` (id:${entry.nativeTagId})` : ''}`).join(' / ')}</span>
      </div>
    `
    list.appendChild(item)
  })
}

function validateConnection() {
  const settings = connectionSettings()
  if (!settings.endpoint) {
    throw new Error('JoyWatcher の内部接続設定を取得できていません')
  }
  if (!Number.isInteger(Number(settings.user_id)) || Number(settings.user_id) <= 0) {
    throw new Error('JoyWatcher の内部 user_id 設定が不正です')
  }
}

function validateReadyToSave() {
  if (!currentDriverId()) {
    throw new Error('TagSel2 の選択結果から接続先IDを取得できていません')
  }
  if (scanGroups.length === 0) {
    throw new Error('TagSel2 でタグを取り込んでから完了してください')
  }
  validateConnection()
}

async function browseTags() {
  validateConnection()

  const settings = connectionSettings()
  const items = await invoke('browse_joywatcher_tags', {
    endpoint: settings.endpoint,
    userId: settings.user_id,
    password: settings.password
  })

  const parsedItems = (Array.isArray(items) ? items : [])
    .map(parseResolvedBrowseItem)
    .filter(Boolean)

  browsedTags = parsedItems
  importBrowsedTags(parsedItems)

  let typeProbeSummary = null
  try {
    typeProbeSummary = await probeImportedTagTypes(parsedItems)
  } catch (error) {
    el('msgErr').textContent = `型確認はスキップしました: ${formatError(error)}`
  }

  renderBrowsedTags()

  if (browsedTags.length > 0) {
    const summary = typeProbeSummary && typeProbeSummary.updatedCount > 0
      ? ` 型確認: bool ${typeProbeSummary.counts.bool}件 / string ${typeProbeSummary.counts.string}件 / number ${typeProbeSummary.counts.number}件`
      : ''
    el('msgOk').textContent = `${browsedTags.length} 件のタグを TagSel2 から取り込み、グループごとに自動登録しました。${summary}`.trim()
  } else {
    el('msgOk').textContent = 'TagSel2 の選択結果は空でした'
  }
}

function upsertGroup() {
  const currentGroup = activeGroup()
  if (!currentGroup) throw new Error('先に左側のグループを選択してください')

  const id = el('groupId').value.trim()
  const node = currentGroup.node || currentDriverId()
  const scanRateMs = Number(el('groupRate').value || 1000)

  if (!id) throw new Error('ScanGroup ID を入力してください')
  if (!node) throw new Error('接続先IDを取得できていません')
  if (scanRateMs < 100) throw new Error('読出し周期は100ms以上にしてください')

  const duplicateIndex = scanGroups.findIndex(
    (group, index) => group.id === id && group.node === node && index !== selectedGroupIndex
  )
  if (duplicateIndex >= 0) {
    throw new Error(`同一接続先・同名グループが既に存在します: ${node} / ${id}`)
  }

  const nextGroup = {
    id,
    node,
    scanRateMs,
    tags: activeGroup()?.tags || []
  }

  scanGroups.splice(selectedGroupIndex, 1, nextGroup)

  renderGroups()
  renderTags()
  refreshSummary()
  el('msgOk').textContent = `選択グループ設定を更新しました: ${id}`
}

function renderReview() {
  const driverId = syncDriverIdFromGroups()
  const detected = countDetectedTypes()
  const resolvedTagIds = countResolvedNativeTagIds()
  const alerts = collectReviewAlerts()
  const warningCount = alerts.filter((alert) => alert.severity !== 'ok').length
  const connectionIds = collectConnectionIds()
  el('reviewConnection').innerHTML = `
    <div class="review-card">
      <span class="review-card-label">接続先ID</span>
      <span class="review-card-value">${driverId || '-'}</span>
    </div>
    <div class="review-card">
      <span class="review-card-label">SelTag2 由来接続先数</span>
      <span class="review-card-value">${connectionIds.length}</span>
    </div>
    <div class="review-card">
      <span class="review-card-label">接続先候補</span>
      <span class="review-card-value">${connectionIds.length > 0 ? connectionIds.join(', ') : '-'}</span>
    </div>
  `
  el('reviewCounts').innerHTML = `
    <div class="review-card">
      <span class="review-card-label">登録グループ</span>
      <span class="review-card-value">${scanGroups.length}</span>
    </div>
    <div class="review-card">
      <span class="review-card-label">登録タグ</span>
      <span class="review-card-value">${totalTagCount()}</span>
    </div>
    <div class="review-card">
      <span class="review-card-label">nativeTagId 解決済み</span>
      <span class="review-card-value">${resolvedTagIds} / ${totalTagCount()}</span>
    </div>
    <div class="review-card">
      <span class="review-card-label">型確認済み</span>
      <span class="review-card-value">${detected.confirmed} / ${totalTagCount()}</span>
    </div>
  `
  el('reviewWarningsCount').textContent = `${warningCount}件`
  el('reviewGroupsCount').textContent = `${scanGroups.length}件`

  const warningList = el('reviewWarnings')
  warningList.innerHTML = ''

  alerts.forEach((alert) => {
    const item = browserWindow.document.createElement('div')
    item.className = `review-warning-item ${alert.severity}`

    const detailsHtml = alert.details.length > 0
      ? `
        <ul class="review-warning-details">
          ${alert.details.map((detail) => `<li>${detail}</li>`).join('')}
        </ul>
      `
      : '<p class="review-warning-empty">追加確認事項はありません。</p>'

    item.innerHTML = `
      <div class="review-warning-head">
        <div class="review-warning-title-wrap">
          <span class="review-warning-title">${alert.title}</span>
          <span class="review-warning-status">${alert.statusLabel}</span>
        </div>
        <span class="review-warning-count">${alert.count}件</span>
      </div>
      <p class="review-warning-summary">${alert.summary}</p>
      ${detailsHtml}
    `

    warningList.appendChild(item)
  })

  const list = el('reviewGroups')
  list.innerHTML = ''

  if (scanGroups.length === 0) {
    list.innerHTML = '<div class="group-name-item muted">登録対象グループはまだありません。TagSel2 でタグ登録してから進んでください。</div>'
    return
  }

  scanGroups.forEach((group) => {
    const item = browserWindow.document.createElement('div')
    item.className = 'group-name-item'
    const detectedCount = group.tags.filter((tag) => Boolean(tag.detectedValueKind)).length
    const resolvedCount = group.tags.filter((tag) => Number.isInteger(Number(tag.nativeTagId)) && Number(tag.nativeTagId) > 0).length
    item.textContent = `${group.id} · ${group.tags.length}タグ · 型確認 ${detectedCount}/${group.tags.length} · ID解決 ${resolvedCount}/${group.tags.length}`
    list.appendChild(item)
  })
}

function buildPayload() {
  validateReadyToSave()

  return {
    schemaVersion: 1,
    driver: {
      id: currentDriverId(),
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
          dataType: tag.dataType || 'f32',
          enabled: tag.enabled,
          unit: tag.unit || null,
          comment: tag.comment || null,
          driverSpec: {
            kind: 'joywatcher',
            node: group.node,
            scanGroup: group.id,
            tagPath: tag.tagPath,
            ...(tag.detectedValueKind
              ? { detectedValueKind: tag.detectedValueKind }
              : {}),
            ...(Number.isInteger(tag.detectedDtype)
              ? { detectedDtype: tag.detectedDtype }
              : {}),
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

    if (el('outOkReview')) {
      el('outOkReview').textContent = '確定しました。ウィンドウを閉じます...'
    } else {
      el('outOk').textContent = '確定しました。ウィンドウを閉じます...'
    }
    browserWindow.setTimeout(() => {
      void closeWindow()
    }, 200)
  } catch (error) {
    if (el('outErrReview')) {
      el('outErrReview').textContent = formatError(error)
    } else {
      el('outErr').textContent = formatError(error)
    }
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

  if (!tauriInvoke) {
    el('driverId').value = ''
    connectionState = {
      endpoint: 'localhost',
      user_id: 1,
      password: '',
      notes: ''
    }
    isEditMode = false
    scanGroups = []
    launchContext = null
    updateModeUi()
    renderGroups()
    renderTags()
    renderBrowsedTags()
    refreshSummary()
    setStep(1)
    el('msgOk').textContent = '静的プレビュー表示中です。TagSel2 呼び出しや保存は Tauri 起動時に利用できます。'
    return
  }

  try {
    launchContext = await invoke('get_driver_ui_launch_context')
    const ctx = launchContext?.context || {}
    const settings = ctx.driverSettings && typeof ctx.driverSettings === 'object'
      ? ctx.driverSettings
      : {}

    isEditMode = Boolean(launchContext?.driverId)
    scanGroups = restoreScanGroups(ctx.scanGroups)

    el('driverId').value = launchContext?.driverId || ''
    connectionState = {
      endpoint: readConnectionSetting(settings, 'endpoint', 'host', 'localhost'),
      user_id: Number(readConnectionSetting(settings, 'user_id', 'userId', 1)),
      password: readConnectionSetting(settings, 'password', 'passwd', ''),
      notes: readConnectionSetting(settings, 'notes', 'memo', '')
    }

    updateModeUi()
    syncDriverIdFromGroups()
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

el('btnStep2Next').addEventListener('click', () => setStep(2))
el('btnStepReviewPrev').addEventListener('click', () => setStep(1))
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
el('btnBrowseTags').addEventListener('click', async () => {
  clearMessages()
  try {
    await browseTags()
  } catch (error) {
    el('msgErr').textContent = formatError(error)
  }
})
el('btnProbeTypes').addEventListener('click', async () => {
  clearMessages()
  setTypeProbeBusy(true)
  try {
    el('msgOk').textContent = 'JWRead で型確認しています...'
    const summary = await probeRegisteredTagTypes()
    el('msgOk').textContent = `型確認を実行しました。bool ${summary.counts.bool}件 / string ${summary.counts.string}件 / number ${summary.counts.number}件`
  } catch (error) {
    el('msgErr').textContent = formatError(error)
  } finally {
    setTypeProbeBusy(false)
  }
})
el('saveButton').addEventListener('click', () => void saveOutput())
browserWindow.document.querySelectorAll('.stepper .step').forEach((button) => {
  button.addEventListener('click', () => {
    const targetStep = Number(button.dataset.step)
    if (targetStep === 1) {
      setStep(1)
      return
    }
    setStep(2)
  })
})

void init()
