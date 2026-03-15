import { useQuery } from '@tanstack/react-query'

export function useCertificate() {
  return useQuery({
    queryKey: ['cert', 'ca'],
    queryFn: async () => {
      const response = await fetch('/api/cert/ca')
      if (!response.ok) {
        throw new Error('Failed to fetch certificate')
      }
      const blob = await response.blob()
      const text = await blob.text()
      return text
    },
  })
}

export function useDownloadCertificate() {
  const { data: certificate, refetch, isFetching } = useCertificate()
  const downloadCertificate = async () => {
    if (!certificate) {
      await refetch()
      return
    }

    const blob = new Blob([certificate], { type: 'application/x-pem' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = 'proxyforge-ca.crt'
    a.click()
    URL.revokeObjectURL(url)
  }

  return { certificate, downloadCertificate, refetch, isFetching }
}
