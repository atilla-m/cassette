export type PlaybackSeekRequest = {
  id: number;
  contextKey: string;
  positionSeconds: number;
};

export type PlaybackPositionSnapshot = {
  contextKey: string;
  revision: number;
};

export function failedSeekRecoveryPosition(
  recoveredPositionSeconds: number | null,
  lastConfirmedPositionSeconds: number,
) {
  if (recoveredPositionSeconds !== null && Number.isFinite(recoveredPositionSeconds) && recoveredPositionSeconds >= 0) {
    return recoveredPositionSeconds;
  }

  return Math.max(0, lastConfirmedPositionSeconds);
}

export class PlaybackSeekCoordinator {
  private contextKey: string | null = null;
  private revision = 0;
  private nextRequestId = 0;
  private latestRequestId = 0;
  private queuedRequest: PlaybackSeekRequest | null = null;
  private activeRequest: PlaybackSeekRequest | null = null;

  reset(contextKey: string | null) {
    this.contextKey = contextKey;
    this.revision += 1;
    this.latestRequestId = ++this.nextRequestId;
    this.queuedRequest = null;
    this.activeRequest = null;
  }

  request(contextKey: string, positionSeconds: number) {
    if (this.contextKey !== contextKey) {
      this.reset(contextKey);
    }

    const request = {
      id: ++this.nextRequestId,
      contextKey,
      positionSeconds,
    };

    this.latestRequestId = request.id;
    this.queuedRequest = request;
    this.revision += 1;

    return request;
  }

  takeNext() {
    if (this.activeRequest || !this.queuedRequest) {
      return null;
    }

    this.activeRequest = this.queuedRequest;
    this.queuedRequest = null;

    return this.activeRequest;
  }

  complete(request: PlaybackSeekRequest) {
    if (this.activeRequest?.id !== request.id) {
      return false;
    }

    this.activeRequest = null;
    this.revision += 1;

    return this.contextKey === request.contextKey
      && this.latestRequestId === request.id
      && this.queuedRequest === null;
  }

  capturePositionUpdate(contextKey: string): PlaybackPositionSnapshot {
    return { contextKey, revision: this.revision };
  }

  allowsPositionUpdate(snapshot: PlaybackPositionSnapshot) {
    return this.contextKey === snapshot.contextKey
      && this.revision === snapshot.revision
      && !this.hasPendingSeek(snapshot.contextKey);
  }

  hasPendingSeek(contextKey: string) {
    return this.activeRequest?.contextKey === contextKey
      || this.queuedRequest?.contextKey === contextKey;
  }
}
