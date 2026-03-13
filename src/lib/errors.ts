/**
 * Error categories matching the Rust AppError enum
 */
export type ErrorCategory =
  | 'fileSystem'
  | 'network'
  | 'mediaProcessing'
  | 'model'
  | 'configuration'
  | 'cancelled'
  | 'validation'
  | 'unknown';

/**
 * Structured error from Rust backend
 */
export interface AppError {
  category: ErrorCategory;
  message: string;
  // Category-specific optional fields
  path?: string;
  url?: string;
  statusCode?: number;
  context?: string;
  modelName?: string;
  key?: string;
  phase?: string;
  field?: string;
}

/**
 * Check if an error object is a structured AppError
 */
export function isAppError(error: unknown): error is AppError {
  return (
    typeof error === 'object' &&
    error !== null &&
    'category' in error &&
    'message' in error
  );
}

/**
 * Extract user-friendly error message from various error types
 */
export function getErrorMessage(error: unknown, fallback = 'Đã xảy ra lỗi không xác định'): string {
  if (isAppError(error)) {
    return error.message;
  }
  
  if (error instanceof Error) {
    return error.message;
  }
  
  if (typeof error === 'string') {
    return error;
  }
  
  return fallback;
}

/**
 * Get detailed error information for debugging
 */
export function getErrorDetails(error: unknown): string {
  if (isAppError(error)) {
    const parts: string[] = [error.message];
    
    if (error.path) parts.push(`Path: ${error.path}`);
    if (error.url) parts.push(`URL: ${error.url}`);
    if (error.statusCode) parts.push(`Status: ${error.statusCode}`);
    if (error.context) parts.push(`Context: ${error.context}`);
    if (error.modelName) parts.push(`Model: ${error.modelName}`);
    if (error.key) parts.push(`Key: ${error.key}`);
    if (error.phase) parts.push(`Phase: ${error.phase}`);
    if (error.field) parts.push(`Field: ${error.field}`);
    
    return parts.join('\n');
  }
  
  if (error instanceof Error) {
    return error.stack ?? error.message;
  }
  
  if (typeof error === 'string') {
    return error;
  }
  
  try {
    return JSON.stringify(error, null, 2);
  } catch {
    return String(error);
  }
}

/**
 * Vietnamese error messages by category
 */
export const ERROR_MESSAGES_VI: Record<ErrorCategory, string> = {
  fileSystem: 'Lỗi hệ thống file',
  network: 'Lỗi kết nối mạng',
  mediaProcessing: 'Lỗi xử lý media',
  model: 'Lỗi model AI',
  configuration: 'Lỗi cấu hình',
  cancelled: 'Đã hủy bỏ',
  validation: 'Lỗi xác thực dữ liệu',
  unknown: 'Lỗi không xác định'
};

/**
 * Get localized error category name
 */
export function getErrorCategoryName(category: ErrorCategory): string {
  return ERROR_MESSAGES_VI[category];
}

/**
 * Format error for display in UI
 */
export interface FormattedError {
  title: string;
  message: string;
  details?: string;
  category?: ErrorCategory;
}

export function formatError(error: unknown): FormattedError {
  if (isAppError(error)) {
    return {
      title: getErrorCategoryName(error.category),
      message: error.message,
      details: getErrorDetails(error),
      category: error.category
    };
  }
  
  return {
    title: 'Lỗi',
    message: getErrorMessage(error),
    details: getErrorDetails(error)
  };
}
